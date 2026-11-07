import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Overview } from "../app/Overview";
import type { Workspace } from "../app/WorkspaceNav";
import "./github.css";

type Account = {
  githubId: number;
  login: string;
  expiresAt: number;
  verifiedAt: number;
};
type AccountStatus = {
  account: Account | null;
  state: "connected" | "expired" | "unavailable" | "absent";
  cleanupPending: boolean;
};
type Flow = {
  sessionId: string;
  stage: "starting" | "pending" | "verifying" | "connected";
  userCode: string | null;
  expiresIn: number;
  retryAfterMs: number;
};

const errors: Record<string, string> = {
  busy: "Hay otra operación de GitHub en curso. Vuelve a intentarlo.",
  stale: "Esta autorización terminó. Inicia una nueva conexión.",
  unavailable:
    "No se pudo completar la conexión. Vuelve a autorizar la cuenta.",
  network:
    "No se pudo contactar con GitHub. Comprueba la conexión y vuelve a intentarlo.",
  protocol:
    "GitHub devolvió una respuesta incompatible. Inicia una nueva autorización.",
  invalid_app:
    "Comprueba que Device Flow esté activado en la aplicación de GitHub.",
  denied: "La autorización se rechazó en GitHub. Puedes iniciar otra conexión.",
  expired: "La autorización venció. Conecta la cuenta de nuevo.",
  cancelled: "Autorización cancelada.",
  secure_store:
    "El almacén seguro no está disponible. Desbloquéalo y vuelve a intentarlo. En Linux se necesita Secret Service y secret-tool.",
  storage:
    "No se pudo guardar el estado de la cuenta. Comprueba el almacenamiento local.",
  different_account:
    "La cuenta autorizada es distinta de la guardada. Usa la misma cuenta para esta conexión.",
  not_connected:
    "GitHub rechazó la sesión guardada. Autoriza la cuenta de nuevo.",
  rate_limited:
    "GitHub limitó las solicitudes. Espera antes de volver a intentarlo.",
};

function message(problem: unknown) {
  return typeof problem === "string"
    ? (errors[problem] ?? errors.unavailable)
    : errors.unavailable;
}

function date(seconds: number) {
  return new Date(seconds * 1000).toLocaleString();
}

export function GithubPage({
  onNavigate,
}: {
  onNavigate: (workspace: Workspace) => void;
}) {
  const [account, setAccount] = useState<AccountStatus>();
  const [flow, setFlow] = useState<Flow | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const connect = useRef<HTMLButtonElement>(null);
  const code = useRef<HTMLElement>(null);
  const restoreFocus = useRef(false);
  const activeSession = useRef<string | null>(null);

  useEffect(() => {
    let active = true;
    void Promise.all([
      invoke<AccountStatus>("github_account"),
      invoke<Flow | null>("github_session"),
    ]).then(
      ([account, session]) => {
        if (!active) return;
        setAccount(account);
        activeSession.current = session?.sessionId ?? null;
        setFlow(session);
      },
      (problem) => {
        if (active) setError(message(problem));
      },
    );
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (!flow) return;
    let active = true;
    const timer = setTimeout(() => {
      void invoke<Flow>("github_poll", { sessionId: flow.sessionId })
        .then(async (next) => {
          if (!active || activeSession.current !== flow.sessionId) return;
          if (next.stage === "connected") {
            activeSession.current = null;
            restoreFocus.current = true;
            setFlow(null);
            setAccount(await invoke<AccountStatus>("github_account"));
            setNotice("Cuenta verificada y guardada en el almacén seguro.");
          } else setFlow(next);
        })
        .catch(async (problem: unknown) => {
          if (!active || activeSession.current !== flow.sessionId) return;
          activeSession.current = null;
          restoreFocus.current = true;
          setFlow(null);
          setError(message(problem));
          setAccount(
            await invoke<AccountStatus>("github_account").catch(
              () => undefined,
            ),
          );
        });
    }, flow.retryAfterMs);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [flow]);

  useEffect(() => {
    if (restoreFocus.current && !flow && !busy) {
      connect.current?.focus();
      restoreFocus.current = false;
    }
  }, [flow, busy]);

  useEffect(() => {
    if (flow?.userCode) code.current?.focus();
  }, [flow?.userCode]);

  async function run(operation: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await operation();
    } catch (problem) {
      setError(message(problem));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Overview
      workspace="github"
      title="GitHub"
      subtitle="Tu cuenta y los repositorios autorizados."
      busy={busy || Boolean(flow)}
      onNavigate={onNavigate}
    >
      {account?.account && (
        <div className="inventory-section">
          <h2>@{account.account.login}</h2>
          <p className="inventory-meta">
            ID de GitHub · {account.account.githubId}
          </p>
          <p>
            {account.state === "connected"
              ? "Sesión guardada"
              : account.state === "expired"
                ? "Sesión vencida"
                : "Credencial sin comprobar"}
          </p>
          <dl className="github-dates">
            <dt>Última verificación</dt>
            <dd>{date(account.account.verifiedAt)}</dd>
            <dt>Vencimiento</dt>
            <dd>{date(account.account.expiresAt)}</dd>
          </dl>
        </div>
      )}
      {flow ? (
        <section
          className="inventory-section"
          aria-labelledby="github-authorization"
        >
          <h2 id="github-authorization">Autoriza en GitHub</h2>
          {flow.userCode ? (
            <>
              <p>
                Introduce este código en{" "}
                <strong>github.com/login/device</strong>.
              </p>
              <code className="github-device-code" tabIndex={-1} ref={code}>
                {flow.userCode}
              </code>
              <p className="inventory-meta">
                Disponible durante {Math.ceil(flow.expiresIn / 60)} min.
                GitPilot espera la autorización.
              </p>
              <button
                className="button primary"
                disabled={busy}
                onClick={() =>
                  void run(() =>
                    invoke("github_open_verification", {
                      sessionId: flow.sessionId,
                    }),
                  )
                }
              >
                Abrir GitHub en el navegador
              </button>
            </>
          ) : (
            <p role="status">Solicitando el código de autorización…</p>
          )}
          <div className="action-row github-cancel">
            <button
              className="button"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await invoke("github_cancel", { sessionId: flow.sessionId });
                  activeSession.current = null;
                  restoreFocus.current = true;
                  setFlow(null);
                  const status = await invoke<AccountStatus>("github_account");
                  setAccount(status);
                  setNotice(
                    status.state === "connected"
                      ? "La cuenta ya quedó conectada."
                      : "Autorización cancelada.",
                  );
                })
              }
            >
              Cancelar autorización
            </button>
          </div>
        </section>
      ) : (
        <section className="inventory-section">
          <h2>
            {account?.account ? "Conexión de la cuenta" : "Conecta tu cuenta"}
          </h2>
          <p className="panel-intro">
            {account?.state === "connected"
              ? "La cuenta está guardada en el almacén seguro del sistema. Puedes comprobar su acceso a GitHub."
              : "Autoriza GitPilot en tu navegador y revisa los permisos antes de confirmar la conexión."}
          </p>
          <div className="action-row">
            <button
              ref={connect}
              className={
                account?.state === "connected" ? "button" : "button primary"
              }
              disabled={busy || !account}
              onClick={() =>
                void run(async () => {
                  const next = await invoke<Flow>("github_start");
                  activeSession.current = next.sessionId;
                  setFlow(next);
                })
              }
            >
              {account?.account ? "Volver a autorizar" : "Conectar con GitHub"}
            </button>
            {account?.state === "connected" && (
              <button
                className="button primary"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    setAccount(await invoke<AccountStatus>("github_check"));
                    setNotice("La cuenta respondió desde GitHub.");
                  })
                }
              >
                Comprobar conexión
              </button>
            )}
            {!account && (
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  void run(async () =>
                    setAccount(await invoke<AccountStatus>("github_account")),
                  )
                }
              >
                Reintentar carga
              </button>
            )}
          </div>
        </section>
      )}
      {account?.cleanupPending && (
        <p role="status" className="inventory-meta">
          Hay una credencial anterior pendiente de retirada. Vuelve a cargar
          esta sección para reintentar.
        </p>
      )}
      {notice && (
        <p role="status" className="notice">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="inventory-conflict">
          {error}
        </p>
      )}
    </Overview>
  );
}
