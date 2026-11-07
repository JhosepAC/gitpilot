import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Overview } from "../app/Overview";
import type { Workspace } from "../app/WorkspaceNav";
import { GithubResources } from "./GithubResources";
import "./github.css";

type Identity = { githubId: number; login: string };
type Account = Identity & {
  expiresAt: number;
  verifiedAt: number;
  state: keyof typeof accountStates;
};
type Accounts = {
  accounts: Account[];
  revocations: Identity[];
  cleanupPending: boolean;
};
type Flow = {
  sessionId: string;
  stage: string;
  userCode: string | null;
  expiresIn: number;
  retryAfterMs: number;
  githubId: number | null;
};

const accountStates = {
  connected: "Sesión guardada",
  expired: "Sesión vencida",
  unavailable: "Almacén seguro sin comprobar",
  refreshing: "Renovación pendiente",
  reauthorize: "Requiere autorización",
  disconnecting: "Retirada pendiente",
};

const errors: Record<string, string> = {
  busy: "Hay otra operación de GitHub en curso. Vuelve a intentarlo.",
  stale: "La cuenta o la autorización cambió. Abre una nueva revisión.",
  unavailable: "No se pudo completar la operación. Vuelve a intentarlo.",
  network:
    "No se pudo contactar con GitHub. Comprueba la conexión y vuelve a intentarlo.",
  protocol: "GitHub devolvió una respuesta incompatible.",
  invalid_app:
    "Comprueba que Device Flow esté activado en la aplicación de GitHub.",
  denied: "La autorización se rechazó en GitHub. Puedes iniciar otra conexión.",
  expired: "La sesión venció. Vuelve a autorizar la cuenta.",
  cancelled: "Autorización cancelada.",
  secure_store:
    "El almacén seguro no está disponible. Desbloquéalo y vuelve a intentarlo.",
  storage:
    "No se pudo guardar el estado de la cuenta. Comprueba el almacenamiento local.",
  different_account:
    "GitHub respondió con otra cuenta. Selecciona la cuenta que elegiste en GitPilot.",
  not_connected: "La sesión requiere una nueva autorización.",
  rate_limited:
    "GitHub limitó las solicitudes. Espera antes de volver a intentarlo.",
  renewal_uncertain:
    "La renovación no se pudo verificar. Vuelve a comprobar o autorizar la cuenta.",
};

function message(problem: unknown) {
  return typeof problem === "string"
    ? (errors[problem] ?? errors.unavailable)
    : errors.unavailable;
}

const readAccounts = () => invoke<Accounts>("github_account");
const date = (seconds: number) => new Date(seconds * 1000).toLocaleString();

function Avatar({ account, large }: { account: Identity; large?: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={large ? "github-avatar github-avatar-large" : "github-avatar"}
    >
      {account.login.charAt(0).toUpperCase()}
      <img
        key={account.githubId}
        src={`https://avatars.githubusercontent.com/u/${account.githubId}?v=4&s=96`}
        alt=""
        referrerPolicy="no-referrer"
        onError={(event) => {
          event.currentTarget.hidden = true;
        }}
      />
    </span>
  );
}

export function GithubPage({
  onNavigate,
}: {
  onNavigate: (workspace: Workspace) => void;
}) {
  const [data, setData] = useState<Accounts>();
  const [selected, setSelected] = useState<number>();
  const [explore, setExplore] = useState(false);
  const [flow, setFlow] = useState<Flow | null>(null);
  const [busy, setBusy] = useState(false);
  const [disconnecting, setDisconnecting] = useState<number>();
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const addButton = useRef<HTMLButtonElement>(null);
  const cancelDisconnectButton = useRef<HTMLButtonElement>(null);
  const disconnectButton = useRef<HTMLButtonElement>(null);
  const checkButton = useRef<HTMLButtonElement>(null);
  const resourcesButton = useRef<HTMLButtonElement>(null);
  const code = useRef<HTMLElement>(null);
  const restoreFocus = useRef<"account" | "disconnect" | "resources">(
    undefined,
  );
  const activeSession = useRef<string | null>(null);
  const inProgress = useRef(false);
  const autoAttempt = useRef<string>(undefined);
  const accounts = data?.accounts ?? [];
  const account =
    accounts.find((item) => item.githubId === selected) ?? accounts[0];
  const id = account?.githubId;
  const state = account?.state;
  const expiresAt = account?.expiresAt;
  const canExplore =
    state === "connected" || state === "expired" || state === "refreshing";

  useEffect(() => {
    let active = true;
    void Promise.all([
      readAccounts(),
      invoke<Flow | null>("github_session"),
    ]).then(
      ([accounts, session]) => {
        if (!active) return;
        setData(accounts);
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
            const accounts = await readAccounts();
            if (!active || activeSession.current !== flow.sessionId) return;
            activeSession.current = null;
            restoreFocus.current = "account";
            setSelected(next.githubId ?? undefined);
            setData(accounts);
            setFlow(null);
            setNotice("Cuenta verificada y guardada en el almacén seguro.");
          } else setFlow(next);
        })
        .catch((problem: unknown) => {
          if (!active || activeSession.current !== flow.sessionId) return;
          activeSession.current = null;
          restoreFocus.current = "account";
          setFlow(null);
          setError(message(problem));
        });
    }, flow.retryAfterMs);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [flow]);

  useEffect(() => {
    if (flow?.userCode) code.current?.focus();
  }, [flow?.userCode]);
  useEffect(() => {
    if (disconnecting) cancelDisconnectButton.current?.focus();
  }, [disconnecting]);
  useEffect(() => {
    if (busy || flow || disconnecting || explore || !restoreFocus.current)
      return;
    const target = {
      disconnect: disconnectButton.current,
      resources:
        resourcesButton.current ?? checkButton.current ?? addButton.current,
      account: checkButton.current ?? addButton.current,
    }[restoreFocus.current];
    target?.focus();
    restoreFocus.current = undefined;
  }, [busy, flow, disconnecting, explore]);

  const run = useCallback(async (operation: () => Promise<void>) => {
    if (inProgress.current) return;
    inProgress.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await operation();
    } catch (problem) {
      setError(message(problem));
      const refreshed = await readAccounts().catch(() => undefined);
      if (refreshed) setData(refreshed);
    } finally {
      inProgress.current = false;
      setBusy(false);
    }
  }, []);

  const check = useCallback(
    (githubId: number, automatic = false) =>
      run(async () => {
        setData(await invoke<Accounts>("github_check", { githubId }));
        setNotice(
          automatic
            ? "Sesión comprobada."
            : "La cuenta respondió desde GitHub.",
        );
      }),
    [run],
  );

  useEffect(() => {
    if (
      !id ||
      !expiresAt ||
      busy ||
      flow ||
      disconnecting ||
      (state !== "connected" && state !== "expired" && state !== "refreshing")
    )
      return;
    const key = `${id}/${expiresAt}/${state}`;
    const delay =
      state === "refreshing"
        ? 0
        : Math.max(0, (expiresAt - 60) * 1000 - Date.now() + 1_000);
    const timer = setTimeout(
      () => {
        if (autoAttempt.current === key || inProgress.current) return;
        autoAttempt.current = key;
        void check(id, true);
      },
      Math.min(delay, 2_147_483_647),
    );
    return () => clearTimeout(timer);
  }, [id, state, expiresAt, busy, flow, disconnecting, check]);

  function start(githubId?: number) {
    return run(async () => {
      const next = await invoke<Flow>("github_start", {
        githubId: githubId ?? null,
      });
      activeSession.current = next.sessionId;
      setExplore(false);
      setFlow(next);
    });
  }

  function cancel(sessionId: string) {
    return run(async () => {
      await invoke("github_cancel", { sessionId });
      activeSession.current = null;
      restoreFocus.current = "account";
      setFlow(null);
      setData(await readAccounts());
      setNotice("Autorización cancelada.");
    });
  }

  function disconnect(githubId: number) {
    return run(async () => {
      try {
        setData(await invoke<Accounts>("github_disconnect", { githubId }));
        setNotice(
          "Sesión retirada de este equipo. Revisa la autorización en GitHub.",
        );
      } finally {
        restoreFocus.current = "account";
        setDisconnecting(undefined);
      }
    });
  }

  const locked = busy || Boolean(flow) || Boolean(disconnecting);
  return (
    <Overview
      workspace="github"
      title="GitHub"
      subtitle="Cuentas y acceso a GitHub."
      busy={locked}
      onNavigate={onNavigate}
    >
      <div className="github-layout">
        <section className="github-accounts" aria-label="Cuentas conectadas">
          <h2>Cuentas</h2>
          {accounts.length > 0 && (
            <ul className="github-account-list">
              {accounts.map((item) => (
                <li key={item.githubId}>
                  <button
                    type="button"
                    aria-pressed={item.githubId === id}
                    disabled={locked}
                    onClick={() => {
                      autoAttempt.current = undefined;
                      setSelected(item.githubId);
                      setError("");
                      setNotice("");
                    }}
                  >
                    <Avatar account={item} />
                    <span className="github-account-name" title={item.login}>
                      {item.login}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <button
            ref={addButton}
            className="button"
            disabled={locked || !data}
            onClick={() => void start()}
          >
            Añadir cuenta
          </button>
        </section>
        <div className="github-detail">
          {flow ? (
            <section aria-labelledby="github-authorization">
              <h2 id="github-authorization">Autoriza en GitHub</h2>
              {flow.userCode ? (
                <>
                  <p>
                    Introduce el código en{" "}
                    <strong>github.com/login/device</strong> con la cuenta que
                    quieres conectar.
                  </p>
                  <code className="github-device-code" tabIndex={-1} ref={code}>
                    {flow.userCode}
                  </code>
                  <p className="inventory-meta">
                    Disponible durante {Math.ceil(flow.expiresIn / 60)} min.
                    GitPilot espera la autorización.
                  </p>
                  <div className="action-row">
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
                      Abrir GitHub
                    </button>
                    <button
                      className="button"
                      disabled={busy}
                      onClick={() => void cancel(flow.sessionId)}
                    >
                      Cancelar autorización
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <p role="status">Solicitando el código de autorización…</p>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() => void cancel(flow.sessionId)}
                  >
                    Cancelar autorización
                  </button>
                </>
              )}
            </section>
          ) : account && explore ? (
            <GithubResources
              key={account.githubId}
              githubId={account.githubId}
              login={account.login}
              onBack={() => {
                restoreFocus.current = "resources";
                setExplore(false);
                void readAccounts().then(setData, () => {});
              }}
            />
          ) : account ? (
            <section aria-labelledby="github-account-title">
              <div className="github-account-heading">
                <Avatar account={account} large />
                <div>
                  <h2 id="github-account-title">@{account.login}</h2>
                  <p className="inventory-meta">
                    ID de GitHub · {account.githubId}
                  </p>
                </div>
              </div>
              <p>{accountStates[account.state]}</p>
              <dl className="github-dates">
                <dt>Última verificación</dt>
                <dd>{date(account.verifiedAt)}</dd>
                <dt>Vencimiento</dt>
                <dd>{date(account.expiresAt)}</dd>
              </dl>
              {disconnecting === id ? (
                <>
                  <p className="inventory-meta">
                    Se retirarán las credenciales de @{account.login} de este
                    equipo.
                  </p>
                  <div className="action-row">
                    <button
                      className="button danger"
                      disabled={busy}
                      onClick={() => void disconnect(account.githubId)}
                    >
                      Confirmar desconexión
                    </button>
                    <button
                      className="button"
                      ref={cancelDisconnectButton}
                      disabled={busy}
                      onClick={() => {
                        restoreFocus.current = "disconnect";
                        setDisconnecting(undefined);
                      }}
                    >
                      Cancelar
                    </button>
                  </div>
                </>
              ) : (
                <div className="action-row">
                  {canExplore && (
                    <button
                      className="button primary"
                      ref={resourcesButton}
                      disabled={busy}
                      onClick={() => {
                        setError("");
                        setNotice("");
                        setExplore(true);
                      }}
                    >
                      Ver instalaciones
                    </button>
                  )}
                  {state !== "reauthorize" && state !== "disconnecting" && (
                    <button
                      className={canExplore ? "button" : "button primary"}
                      ref={checkButton}
                      disabled={busy}
                      onClick={() => void check(account.githubId)}
                    >
                      Comprobar conexión
                    </button>
                  )}
                  {state !== "disconnecting" && (
                    <button
                      className={
                        state === "reauthorize" ? "button primary" : "button"
                      }
                      disabled={busy}
                      onClick={() => void start(account.githubId)}
                    >
                      Volver a autorizar
                    </button>
                  )}
                  <button
                    className="button danger"
                    ref={disconnectButton}
                    disabled={busy}
                    onClick={() => setDisconnecting(account.githubId)}
                  >
                    {state === "disconnecting"
                      ? "Reintentar retirada"
                      : "Desconectar"}
                  </button>
                </div>
              )}
            </section>
          ) : (
            <section>
              <h2>Conecta tu cuenta</h2>
              <p className="panel-intro">
                Autoriza GitPilot en tu navegador y revisa los permisos antes de
                confirmar la conexión.
              </p>
              {!data && (
                <button
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void run(async () => setData(await readAccounts()))
                  }
                >
                  Reintentar carga
                </button>
              )}
            </section>
          )}
        </div>
      </div>
      {data?.revocations.map((item) => (
        <section key={item.githubId} className="inventory-section">
          <h2>Revocación pendiente · @{item.login}</h2>
          <p className="inventory-meta">
            Abre las autorizaciones de GitHub con esta cuenta y revoca GitPilot.
          </p>
          <div className="action-row">
            <button
              className="button"
              disabled={locked}
              onClick={() =>
                void run(() => invoke("github_open_authorizations"))
              }
            >
              Abrir autorizaciones
            </button>
            <button
              className="button"
              disabled={locked}
              onClick={() =>
                void run(async () => {
                  setData(
                    await invoke<Accounts>("github_revocation_reviewed", {
                      githubId: item.githubId,
                    }),
                  );
                })
              }
            >
              Marcar como revisada
            </button>
          </div>
        </section>
      ))}
      {data?.cleanupPending && (
        <p role="status" className="inventory-meta">
          Hay credenciales anteriores pendientes de retirada. Vuelve a cargar
          esta sección para reintentar.
        </p>
      )}
      {busy && (
        <p role="status" className="inventory-meta">
          Procesando la operación de GitHub…
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
