import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { inventoryApi, inventoryError } from "./inventoryApi";
import type { NativeOperation } from "./inventoryApi";

type ProbeKind =
  | "ssh"
  | "ssh_configured"
  | "https"
  | "https_credentials"
  | "signing"
  | "open_pgp";
type ProbePreview = {
  confirmation: string;
  host: string | null;
  via: string | null;
  key: string | null;
  kind: ProbeKind;
};
type Mode = "warn" | "block";
type HookPreview = {
  target: string;
  identity: string;
  mode: Mode;
  confirmation: string;
};

const probes = {
  ssh: {
    label: "Revisar prueba SSH",
    description:
      "Prueba de lectura con OpenSSH directo. El host debe ser conocido por el cliente.",
    success: "El remoto respondió por SSH.",
  },
  ssh_configured: {
    label: "Revisar ruta SSH configurada",
    description:
      "Consulta el remoto con la ruta y las claves configuradas. OpenSSH exige que cada host sea conocido.",
    success: "El remoto respondió por la ruta SSH configurada.",
  },
  https: {
    label: "Revisar prueba HTTPS",
    description:
      "Prueba de lectura anónima por HTTPS. Un repositorio privado puede requerir credenciales.",
    success: "El remoto respondió a la lectura anónima por HTTPS.",
  },
  https_credentials: {
    label: "Revisar HTTPS con credenciales",
    description:
      "Consulta el remoto con credenciales disponibles en un helper compatible. La prueba se ejecuta sin abrir un inicio de sesión.",
    success: "El remoto respondió a la lectura HTTPS con credenciales.",
  },
  signing: {
    label: "Revisar prueba de firma SSH",
    description:
      "Firma y comprueba un desafío temporal con la clave pública configurada y su clave cargada en el agente.",
    success: "El agente produjo una firma SSH válida para el desafío.",
  },
  open_pgp: {
    label: "Revisar prueba de firma OpenPGP",
    description:
      "GnuPG firma un desafío temporal con la clave indicada y verifica la firma en el equipo.",
    success: "GnuPG produjo una firma OpenPGP válida para el desafío.",
  },
};

const probeErrors: Record<string, string> = {
  unsupported:
    "El destino, la configuración o la referencia de clave requieren revisión manual.",
  stale: "El destino o la clave cambió. Abre una nueva revisión.",
  unavailable:
    "No se pudo preparar la prueba con las herramientas disponibles.",
  timed_out: "La prueba superó el tiempo de espera.",
  failed:
    "La prueba falló. Comprueba el acceso, el host conocido o la disponibilidad de la clave según corresponda.",
  credentials_unavailable:
    "El helper no devolvió credenciales compatibles. Comprueba el acceso con Git y vuelve a intentarlo.",
  signing_unavailable:
    "GnuPG no pudo firmar con la clave indicada. Comprueba que la clave privada esté disponible y desbloqueada.",
};

export function ProbeControl({
  id,
  kinds,
  remote,
}: {
  id: number;
  kinds: ProbeKind[];
  remote?: string;
}) {
  const [preview, setPreview] = useState<ProbePreview>();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState("");
  const [error, setError] = useState("");
  const confirm = useRef<HTMLButtonElement>(null);
  const reviews = useRef<Partial<Record<ProbeKind, HTMLButtonElement | null>>>(
    {},
  );
  const restoreFocus = useRef<ProbeKind>(undefined);
  useEffect(() => {
    if (busy) return;
    if (preview) confirm.current?.focus();
    else if (restoreFocus.current) {
      reviews.current[restoreFocus.current]?.focus();
      restoreFocus.current = undefined;
    }
  }, [busy, preview]);

  async function run(kind: ProbeKind, confirmation?: string) {
    setBusy(true);
    setError("");
    setResult("");
    try {
      if (confirmation) {
        await invoke("verify_inspector_probe", {
          id,
          kind,
          remote,
          confirmation,
        });
        setResult(probes[kind].success);
        restoreFocus.current = kind;
        setPreview(undefined);
      } else {
        setPreview(
          await invoke<ProbePreview>("preview_inspector_probe", {
            id,
            kind,
            remote,
          }),
        );
      }
    } catch (problem) {
      setError(
        typeof problem === "string"
          ? (probeErrors[problem] ?? probeErrors.unavailable)
          : probeErrors.unavailable,
      );
      restoreFocus.current = kind;
      setPreview(undefined);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="inspector-action">
      {preview ? (
        <>
          {preview.host && (
            <p>
              <strong>{preview.host}</strong>
            </p>
          )}
          {preview.via && (
            <p className="inventory-meta">Salto SSH · {preview.via}</p>
          )}
          {preview.key && (
            <p>
              <code>{preview.key}</code>
            </p>
          )}
          <p className="inventory-meta">{probes[preview.kind].description}</p>
          <div className="action-row">
            <button
              className="button primary"
              ref={confirm}
              disabled={busy}
              onClick={() => void run(preview.kind, preview.confirmation)}
            >
              Ejecutar prueba
            </button>
            <button
              className="button"
              disabled={busy}
              onClick={() => {
                restoreFocus.current = preview.kind;
                setPreview(undefined);
              }}
            >
              Cancelar
            </button>
          </div>
        </>
      ) : (
        <div className="action-row">
          {kinds.map((kind) => (
            <button
              className="button"
              disabled={busy}
              key={kind}
              ref={(button) => {
                reviews.current[kind] = button;
              }}
              onClick={() => void run(kind)}
            >
              {probes[kind].label}
            </button>
          ))}
        </div>
      )}
      {(busy || result) && (
        <p role="status" className="inventory-meta">
          {busy ? "Comprobando…" : result}
        </p>
      )}
      {error && (
        <p role="alert" className="inventory-conflict">
          {error}
        </p>
      )}
    </div>
  );
}

export function RemoteProbes({
  id,
  remote,
  transport,
  host,
  configuredSsh,
  credentials,
}: {
  id: number;
  remote: string;
  transport: string;
  host: string | null;
  configuredSsh: boolean;
  credentials: boolean;
}) {
  const direct = ["github.com", "gitlab.com", "bitbucket.org"].includes(
    host ?? "",
  );
  const kinds: ProbeKind[] = [];
  if (transport === "ssh") {
    if (direct) kinds.push("ssh");
    if (host && configuredSsh) kinds.push("ssh_configured");
  } else if (transport === "https" && direct) {
    kinds.push("https");
    if (credentials) kinds.push("https_credentials");
  }
  return kinds.length ? (
    <ProbeControl id={id} remote={remote} kinds={kinds} />
  ) : null;
}

export function HookControl({ id }: { id: number }) {
  const [operation, setOperation] = useState<NativeOperation | null>();
  const [mode, setMode] = useState<Mode>("warn");
  const [preview, setPreview] = useState<HookPreview>();
  const [remove, setRemove] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const confirmationButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (preview || remove) confirmationButton.current?.focus();
  }, [preview, remove]);

  useEffect(() => {
    let active = true;
    void invoke<NativeOperation | null>("pre_commit_status", { id }).then(
      (state) => {
        if (active) setOperation(state);
      },
      (problem) => {
        if (active) setError(inventoryError(problem));
      },
    );
    return () => {
      active = false;
    };
  }, [id]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (problem) {
      setError(
        problem === "unsafe_target" || problem === "active_operation"
          ? "El destino requiere revisión. La instalación necesita un repositorio ordinario, el directorio de hooks predeterminado y un pre-commit libre."
          : inventoryError(problem),
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="inventory-section">
      <h3>Pre-commit</h3>
      <p className="inventory-meta">
        Compara el autor y el committer con la identidad revisada al instalar el
        hook.
      </p>
      {operation ? (
        <div className="inspector-action">
          <p>
            {operation.state === "applied"
              ? "Hook instalado"
              : "Recuperación pendiente"}{" "}
            · {operation.target}
          </p>
          {operation.confirmation &&
            (remove ? (
              <>
                <p className="inventory-meta">
                  GitPilot comprobará el archivo antes de retirarlo.
                </p>
                <div className="action-row">
                  <button
                    className="button danger"
                    ref={confirmationButton}
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        if (!operation.confirmation) return;
                        try {
                          await inventoryApi.confirmNative(
                            operation.confirmation,
                            true,
                            {
                              operationId: operation.id,
                            },
                          );
                        } catch (problem) {
                          setOperation(
                            await invoke<NativeOperation | null>(
                              "pre_commit_status",
                              { id },
                            ),
                          );
                          setRemove(false);
                          throw problem;
                        }
                        setOperation(null);
                        setRemove(false);
                        setNotice("Hook retirado y verificado.");
                      })
                    }
                  >
                    Confirmar retirada
                  </button>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() => setRemove(false)}
                  >
                    Cancelar
                  </button>
                </div>
              </>
            ) : (
              <button
                className="button"
                disabled={busy}
                onClick={() => setRemove(true)}
              >
                Revisar retirada
              </button>
            ))}
          {operation.state === "recovery_required" && (
            <p className="inventory-meta">
              Consulta el destino y el registro en Actividad.
            </p>
          )}
        </div>
      ) : (
        operation === null &&
        (preview ? (
          <div className="inspector-action">
            <p>
              <strong>
                {preview.mode === "warn"
                  ? "Solo aviso"
                  : "Bloqueo ante discrepancia"}
              </strong>
            </p>
            <p>{preview.target}</p>
            <p className="inventory-meta">{preview.identity}</p>
            <div className="action-row">
              <button
                className="button primary"
                ref={confirmationButton}
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    await invoke("confirm_pre_commit", {
                      id,
                      mode: preview.mode,
                      confirmation: preview.confirmation,
                    });
                    setOperation(
                      await invoke<NativeOperation | null>(
                        "pre_commit_status",
                        { id },
                      ),
                    );
                    setPreview(undefined);
                    setNotice("Hook instalado y verificado.");
                  })
                }
              >
                Confirmar instalación
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() => setPreview(undefined)}
              >
                Cancelar
              </button>
            </div>
          </div>
        ) : (
          <>
            <label className="field inventory-select">
              Acción ante una discrepancia
              <select
                value={mode}
                disabled={busy}
                onChange={(event) => setMode(event.currentTarget.value as Mode)}
              >
                <option value="warn">Mostrar aviso</option>
                <option value="block">Bloquear el commit</option>
              </select>
            </label>
            <button
              className="button"
              disabled={busy}
              onClick={() =>
                void run(async () =>
                  setPreview(
                    await invoke<HookPreview>("preview_pre_commit", {
                      id,
                      mode,
                    }),
                  ),
                )
              }
            >
              Revisar instalación
            </button>
          </>
        ))
      )}
      {(busy || notice) && (
        <p role="status" className="inventory-meta">
          {busy ? "Comprobando hook…" : notice}
        </p>
      )}
      {error && (
        <p role="alert" className="inventory-conflict">
          {error}
        </p>
      )}
    </div>
  );
}
