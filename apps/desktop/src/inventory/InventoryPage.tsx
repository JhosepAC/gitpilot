import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Brand } from "../app/Brand";
import { PathPicker } from "../app/PathPicker";
import { useLanguage } from "../app/language";
import { WorkspaceNav } from "../app/WorkspaceNav";
import type { Workspace } from "../app/WorkspaceNav";
import {
  ConfigPreview,
  NativeOperationPanel,
  configFindings,
} from "./ConfigPreview";
import { RepositoryDiagnosis } from "./RepositoryDiagnosis";
import { RepositoryInspector } from "./RepositoryInspector";
import { ScanPreferencesPanel } from "./ScanPreferencesPanel";
import type { InspectorData } from "./RepositoryInspector";
import {
  folderName,
  inventoryApi,
  inventoryError,
  loadInventory,
} from "./inventoryApi";
import type {
  Directory,
  AssignmentPreview,
  LocalPreview,
  NativeOperation,
  NativePreview,
  Repository,
  ScanProgress,
  ScanReport,
} from "./inventoryApi";
import "./inventory.css";

const kinds = {
  ordinary: "Repositorio",
  worktree: "Worktree vinculado",
  separate: "Submódulo o Git dir separado",
  bare: "Repositorio bare",
};

const scanLimitMessages = {
  time: "Se alcanzó el tiempo máximo.",
  entries: "Se alcanzó el límite de entradas.",
  depth: "Se alcanzó la profundidad máxima.",
  repositories: "Se alcanzó el límite de repositorios.",
};

function scanSummary({
  partial,
  cancelled,
  found,
  limits,
  skipped,
}: ScanReport) {
  const state = cancelled
    ? "Escaneo cancelado."
    : partial
      ? "Escaneo parcial."
      : "Escaneo completo.";
  const verified =
    found === 1
      ? "Se verificó 1 repositorio."
      : `Se verificaron ${found} repositorios.`;
  const details = limits.map((limit) => scanLimitMessages[limit]);
  if (skipped > 0) {
    details.push(
      skipped === 1
        ? "Quedó 1 ruta sin verificar."
        : `Quedaron ${skipped} rutas sin verificar.`,
    );
  }
  return [state, verified, ...details].join(" ");
}

type ScanRun = {
  id: string;
  progress?: ScanProgress;
  cancellation: "ready" | "pending" | "finishing";
};

function scanStatus({ progress, cancellation }: ScanRun) {
  if (progress?.phase === "saving" || cancellation === "finishing")
    return "Guardando resultados…";
  if (cancellation === "pending") return "Cancelando escaneo…";
  return progress ? "Escaneando repositorios…" : "Preparando escaneo…";
}

export function InventoryPage({
  onNavigate,
}: {
  onNavigate: (workspace: Workspace) => void;
}) {
  const { t } = useLanguage();
  const [view, setView] = useState<Awaited<ReturnType<typeof loadInventory>>>();
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [mode, setMode] = useState<
    "add" | "exclude" | "remove" | "preferences" | null
  >(null);
  const [activeScan, setActiveScan] = useState<ScanRun>();
  const [preview, setPreview] = useState<NativePreview>();
  const [assignment, setAssignment] = useState<{
    id: number;
    repository: boolean;
    plan: AssignmentPreview;
  }>();
  const [nativeOperation, setNativeOperation] = useState<NativeOperation>();
  const [localPreview, setLocalPreview] = useState<LocalPreview>();
  const [inspector, setInspector] = useState<InspectorData>();
  const [path, setPath] = useState("");
  const panel = useRef<HTMLElement>(null);
  const pathInput = useRef<HTMLInputElement>(null);
  const localReviewTitle = useRef<HTMLHeadingElement>(null);
  const assignmentTitle = useRef<HTMLHeadingElement>(null);
  const localReviewButton = useRef<HTMLButtonElement>(null);
  const previewButton = useRef<HTMLButtonElement>(null);
  const inspectorButton = useRef<HTMLButtonElement>(null);
  const preferencesButton = useRef<HTMLButtonElement>(null);
  const scanButton = useRef<HTMLButtonElement>(null);
  const cancelScanButton = useRef<HTMLButtonElement>(null);
  const activeScanId = useRef<string>(undefined);
  const restoreScanFocus = useRef(false);
  const directory = view?.directory;
  const repository = view?.repository;
  const configOpen = Boolean(
    preview || nativeOperation || localPreview || inspector || assignment,
  );
  const locked = Boolean(busy || mode || configOpen || activeScan);
  const canCancelScan =
    activeScan?.progress?.phase === "walking" &&
    activeScan.cancellation === "ready";

  useEffect(() => {
    let mounted = true;
    void loadInventory().then(
      (next) => {
        if (mounted) setView(next);
      },
      (problem) => {
        if (mounted) setError(inventoryError(problem));
      },
    );
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    panel.current?.scrollTo({ top: 0 });
  }, [directory?.id, repository?.id, mode, configOpen]);

  useEffect(() => {
    pathInput.current?.focus();
  }, [mode]);

  useEffect(() => {
    if (localPreview) localReviewTitle.current?.focus({ preventScroll: true });
  }, [localPreview]);
  useEffect(() => {
    if (assignment) assignmentTitle.current?.focus({ preventScroll: true });
  }, [assignment]);

  useEffect(() => {
    if (canCancelScan) cancelScanButton.current?.focus({ preventScroll: true });
  }, [canCancelScan]);

  useEffect(() => {
    if (!activeScan && restoreScanFocus.current) {
      restoreScanFocus.current = false;
      scanButton.current?.focus({ preventScroll: true });
    }
  }, [activeScan]);

  async function run(message: string, operation: () => Promise<void>) {
    setBusy(message);
    setError("");
    setNotice("");
    try {
      await operation();
    } catch (problem) {
      setError(inventoryError(problem));
    } finally {
      setBusy("");
    }
  }

  async function refresh(directoryId = directory?.id, repositoryId?: number) {
    setView(await loadInventory(directoryId, repositoryId));
  }

  function closeConfig() {
    setPreview(undefined);
    setNativeOperation(undefined);
    requestAnimationFrame(() => previewButton.current?.focus());
  }

  async function openConfig() {
    const operation = await inventoryApi.nativeStatus();
    if (operation) setNativeOperation(operation);
    else setPreview(await inventoryApi.preview());
  }

  function confirmConfig(confirmation: string, restore: boolean) {
    void run(
      restore ? "Restaurando configuración…" : "Aplicando y verificando…",
      async () => {
        try {
          await inventoryApi.confirmNative(confirmation, restore);
        } catch (problem) {
          const state = await inventoryApi.nativeStatus().catch(() => null);
          if (state) {
            setNativeOperation(state);
            setPreview(undefined);
          } else if (problem === "stale_preview") {
            closeConfig();
          }
          throw problem;
        }
        if (restore) {
          closeConfig();
          setNotice("Configuración anterior restaurada y verificada.");
        } else {
          setPreview(undefined);
          setNativeOperation((await inventoryApi.nativeStatus()) ?? undefined);
          setNotice("Configuración aplicada y verificada con Git.");
        }
        await refresh(directory?.id, repository?.id);
      },
    );
  }

  function confirmLocal() {
    if (!localPreview || !repository) return;
    void run("Aplicando la identidad local…", async () => {
      try {
        await inventoryApi.confirmNative(localPreview.confirmation, false, {
          repositoryId: repository.id,
        });
        setNotice(
          "Identidad local verificada. Puedes restaurarla desde Actividad.",
        );
        await refresh(directory?.id, repository.id);
      } finally {
        setLocalPreview(undefined);
      }
    });
  }

  function openPathForm(action: "add" | "exclude") {
    setPath("");
    setError("");
    setNotice("");
    setMode(action);
  }

  function savePath(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void run("Guardando…", async () => {
      if (mode === "add") {
        const id = await inventoryApi.add(path);
        setMode(null);
        await refresh(id);
        setNotice("Directorio registrado.");
      } else if (mode === "exclude" && directory) {
        await inventoryApi.exclude(directory, path, true);
        setMode(null);
        await refresh();
        setNotice(
          "Subcarpeta excluida. Vuelve a escanear para actualizar la lista.",
        );
      }
    });
  }

  function scan(root: Directory) {
    if (activeScanId.current) return;
    const id = Array.from(crypto.getRandomValues(new Uint32Array(4)), (part) =>
      part.toString(16).padStart(8, "0"),
    ).join("");
    activeScanId.current = id;
    setActiveScan({ id, cancellation: "ready" });
    void run("", async () => {
      try {
        const report = await inventoryApi.scan(root.id, id, (progress) => {
          setActiveScan((current) =>
            current?.id === id ? { ...current, progress } : current,
          );
        });
        await refresh(root.id);
        setError("");
        setNotice(scanSummary(report));
      } finally {
        activeScanId.current = undefined;
        restoreScanFocus.current = true;
        setActiveScan(undefined);
      }
    });
  }

  async function cancelScan() {
    if (!canCancelScan || !activeScan) return;
    const { id } = activeScan;
    setActiveScan({ ...activeScan, cancellation: "pending" });
    try {
      const accepted = await inventoryApi.cancelScan(id);
      if (!accepted) {
        setActiveScan((current) =>
          current?.id === id
            ? { ...current, cancellation: "finishing" }
            : current,
        );
      }
    } catch (problem) {
      if (activeScanId.current !== id) return;
      setActiveScan((current) =>
        current?.id === id ? { ...current, cancellation: "ready" } : current,
      );
      setError(inventoryError(problem));
    }
  }

  function openScanPreferences() {
    setError("");
    setNotice("");
    setMode("preferences");
  }

  function assign(target: Directory | Repository, value: string) {
    void run("Preparando asignación…", async () => {
      await inventoryApi.assign(
        target,
        "kind" in target,
        value ? Number(value) : null,
      );
      await refresh(directory?.id, repository?.id);
      setNotice(
        "Regla guardada. Revisa y confirma los cambios para aplicarla a Git.",
      );
      const isRepository = "kind" in target;
      setAssignment({
        id: target.id,
        repository: isRepository,
        plan: await inventoryApi.previewAssignment(target.id, isRepository),
      });
    });
  }

  function identitySelect(target: Directory | Repository, label: string) {
    return (
      <div className="inventory-select">
        <label className="field">
          {label}
          <select
            value={target.identityId ?? ""}
            disabled={locked}
            onChange={(event) => assign(target, event.currentTarget.value)}
          >
            <option value="">Heredar identidad</option>
            {view?.profiles.map((profile) => (
              <option
                key={profile.id}
                value={profile.id}
                disabled={
                  "present" in target &&
                  !target.present &&
                  target.identityId !== profile.id
                }
              >
                {profile.label}
              </option>
            ))}
          </select>
        </label>
        <button
          className="button"
          disabled={locked}
          onClick={() =>
            void run("Preparando asignación…", async () => {
              const isRepository = "kind" in target;
              setAssignment({
                id: target.id,
                repository: isRepository,
                plan: await inventoryApi.previewAssignment(
                  target.id,
                  isRepository,
                ),
              });
            })
          }
        >
          Revisar aplicación del perfil
        </button>
      </div>
    );
  }

  return (
    <main className="identity-shell">
      <header className="identity-heading">
        <div>
          <h1 id="workspace-title" tabIndex={-1}>
            {t("Directorios", "Directories")}
          </h1>
          <p>
            {t(
              "Repositorios y reglas de identidad locales.",
              "Local repositories and identity rules.",
            )}
          </p>
        </div>
        <div className="action-row">
          {view && !mode && !configOpen && (
            <button
              className="button"
              ref={previewButton}
              disabled={locked}
              onClick={() => void run("Consultando configuración…", openConfig)}
            >
              {t("Vista previa de Git", "Git preview")}
            </button>
          )}
          {error && (
            <button
              className="button"
              disabled={locked}
              onClick={() =>
                void run("Actualizando…", () =>
                  refresh(directory?.id, repository?.id),
                )
              }
            >
              {t("Actualizar vista", "Refresh view")}
            </button>
          )}
          {view && !directory && !mode && !configOpen && (
            <button
              className="button"
              ref={preferencesButton}
              disabled={locked}
              onClick={openScanPreferences}
            >
              {t("Preferencias", "Preferences")}
            </button>
          )}
          {view && !mode && !configOpen && (
            <button
              className={directory ? "button" : "button primary"}
              disabled={locked}
              onClick={() => openPathForm("add")}
            >
              {t("Añadir directorio", "Add directory")}
            </button>
          )}
        </div>
      </header>
      <aside
        className="identity-sidebar"
        aria-label={t("Directorios registrados", "Registered directories")}
      >
        <Brand />
        <WorkspaceNav
          current="directories"
          onNavigate={onNavigate}
          disabled={locked}
        />
        <div className="list-heading">
          <h2>{t("Directorios", "Directories")}</h2>
          {view && <span>{view.directories.length}</span>}
        </div>
        {view?.directories.length ? (
          <ul className="profile-list">
            {view.directories.map((root) => (
              <li key={root.id}>
                <button
                  type="button"
                  className="profile-choice directory-choice"
                  title={root.path}
                  aria-label={`${folderName(root.path)} - ${root.path}`}
                  aria-current={root.id === directory?.id ? "true" : undefined}
                  disabled={locked}
                  onClick={() =>
                    void run("Cargando directorio…", () => refresh(root.id))
                  }
                >
                  <svg
                    className="directory-choice-icon"
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.7"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                    focusable="false"
                  >
                    <path d="M3 7V4h6l2 3h10v13H3V7Z" />
                  </svg>
                  <span className="profile-label">{folderName(root.path)}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="profile-empty">
            {view
              ? t("Aún no hay directorios.", "No directories yet.")
              : t("Inventario local", "Local inventory")}
          </p>
        )}
        <footer className="sidebar-foot">
          {t("Almacenamiento local", "Local storage")}
        </footer>
      </aside>
      <section
        className="identity-panel inventory-panel"
        aria-label="Inventario"
        lang="es"
        ref={panel}
      >
        <div className="identity-feedback">
          <div role="status">{busy || notice}</div>
          {error && <p role="alert">{error}</p>}
        </div>
        {inspector && repository ? (
          <RepositoryInspector
            data={inspector}
            name={folderName(repository.path)}
            id={repository.id}
            path={repository.path}
            ordinary={repository.kind === "ordinary"}
            onBack={() => {
              setInspector(undefined);
              requestAnimationFrame(() => inspectorButton.current?.focus());
            }}
          />
        ) : localPreview ? (
          <>
            <button
              className="button inventory-back"
              disabled={Boolean(busy)}
              onClick={() => {
                setLocalPreview(undefined);
                requestAnimationFrame(() => localReviewButton.current?.focus());
              }}
            >
              Volver al repositorio
            </button>
            <h2 className="preview-title" tabIndex={-1} ref={localReviewTitle}>
              Confirmar identidad local
            </h2>
            <p className="panel-intro">
              GitPilot respaldará este archivo antes de añadir los valores
              indicados. Después verificará la configuración, el autor y el
              committer.
            </p>
            <div className="commit-author preview-destinations">
              <h3>Archivo del repositorio</h3>
              <p>{localPreview.target}</p>
              <pre className="preview-diff">{localPreview.append}</pre>
            </div>
            <button
              className="button primary preview-actions"
              disabled={Boolean(busy)}
              onClick={confirmLocal}
            >
              Confirmar cambio local
            </button>
          </>
        ) : nativeOperation ? (
          <NativeOperationPanel
            operation={nativeOperation}
            busy={Boolean(busy)}
            onBack={closeConfig}
            onRestore={(confirmation) => confirmConfig(confirmation, true)}
          />
        ) : assignment ? (
          <>
            <h2 ref={assignmentTitle} tabIndex={-1}>
              Aplicar perfil a Git
            </h2>
            <p className="panel-intro">
              El perfil sustituirá el autor, la autenticación SSH y la firma en
              los archivos siguientes. Cada archivo tendrá respaldo y podrás
              restaurarlo desde Actividad.
            </p>
            <p className="field-help">
              Sin clave de autenticación, SSH quedará bloqueado. Las variables
              de entorno y las opciones de Git de otras terminales pueden tener
              prioridad.
            </p>
            {assignment.plan.repositories.map(({ id, path, preview }) => (
              <details key={id} className="inventory-disclosure">
                <summary>
                  {path} · {preview.name} &lt;{preview.email}&gt;
                </summary>
                <p className="code-value">{preview.target}</p>
                <pre className="preview-diff">{preview.append}</pre>
              </details>
            ))}
            <details className="inventory-disclosure">
              <summary>
                Reglas para repositorios futuros · configuración global
              </summary>
              <p className="code-value">{assignment.plan.global.globalPath}</p>
              <pre className="preview-diff">
                {assignment.plan.global.globalAppend}
              </pre>
            </details>
            {assignment.plan.global.findings.length > 0 && (
              <ul className="preview-findings">
                {assignment.plan.global.findings.map(
                  ({ code, path }, index) => (
                    <li key={index}>
                      {configFindings[code] ??
                        "La configuración requiere revisión."}
                      {path && <p className="code-value">{path}</p>}
                    </li>
                  ),
                )}
              </ul>
            )}
            {!assignment.plan.confirmation && (
              <p role="alert">
                La regla está guardada, pero la configuración requiere revisión.
                Abre la vista previa de Git para consultar los límites
                detectados.
              </p>
            )}
            <div className="action-row">
              <button
                className="button primary"
                disabled={Boolean(busy) || !assignment.plan.confirmation}
                onClick={() =>
                  void run("Aplicando y verificando perfil…", async () => {
                    try {
                      await inventoryApi.confirmAssignment(
                        assignment.id,
                        assignment.repository,
                        assignment.plan.confirmation!,
                      );
                      setAssignment(undefined);
                      setNotice(
                        "Configuración del perfil aplicada y verificada con Git.",
                      );
                    } catch (problem) {
                      setAssignment(undefined);
                      setNotice(
                        "La aplicación no se completó. Revisa el diagnóstico y las operaciones verificadas en Actividad antes de reintentar.",
                      );
                      throw problem;
                    } finally {
                      await refresh(directory?.id, repository?.id);
                    }
                  })
                }
              >
                Aplicar perfil
              </button>
              <button
                className="button"
                disabled={Boolean(busy)}
                onClick={() => {
                  setAssignment(undefined);
                  requestAnimationFrame(() => panel.current?.focus());
                }}
              >
                Volver al inventario
              </button>
            </div>
          </>
        ) : preview ? (
          <ConfigPreview
            preview={preview}
            busy={Boolean(busy)}
            onBack={closeConfig}
            onConfirm={(confirmation) => confirmConfig(confirmation, false)}
          />
        ) : mode === "preferences" ? (
          <ScanPreferencesPanel
            onBack={() => {
              setMode(null);
              requestAnimationFrame(() => preferencesButton.current?.focus());
            }}
          />
        ) : mode === "add" || mode === "exclude" ? (
          <>
            <h2>
              {mode === "add" ? "Registrar directorio" : "Excluir subcarpeta"}
            </h2>
            <p className="panel-intro">
              {mode === "add"
                ? "Registra la ruta de una carpeta con tus proyectos."
                : "La subcarpeta y sus descendientes se omitirán en el siguiente escaneo de este directorio."}
            </p>
            <form className="identity-form" onSubmit={savePath}>
              <PathPicker
                label="Directorio"
                inputRef={pathInput}
                value={path}
                onChange={setPath}
                directory
                disabled={Boolean(busy)}
              />
              <div className="action-row">
                <button
                  className="button primary"
                  disabled={Boolean(busy) || !path}
                >
                  {mode === "add" ? "Registrar" : "Excluir"}
                </button>
                <button
                  type="button"
                  className="button"
                  disabled={Boolean(busy)}
                  onClick={() => setMode(null)}
                >
                  Cancelar
                </button>
              </div>
            </form>
          </>
        ) : mode === "remove" && directory ? (
          <>
            <h2>Retirar directorio</h2>
            <p className="panel-intro">
              Retira este directorio del inventario y elimina sus reglas
              locales. Sus archivos seguirán en su ubicación.
            </p>
            <div className="action-row">
              <button
                className="button"
                disabled={Boolean(busy)}
                onClick={() => setMode(null)}
              >
                Cancelar
              </button>
              <button
                className="button danger"
                disabled={Boolean(busy)}
                onClick={() =>
                  void run("Retirando directorio…", async () => {
                    await inventoryApi.remove(directory);
                    setMode(null);
                    await refresh();
                    setNotice("Directorio retirado.");
                  })
                }
              >
                Retirar del inventario
              </button>
            </div>
          </>
        ) : repository && view.diagnosis ? (
          <>
            <button
              className="button inventory-back"
              disabled={locked}
              onClick={() => void run("Cargando directorio…", () => refresh())}
            >
              Volver al directorio
            </button>
            <div className="detail-heading">
              <h2>{folderName(repository.path)}</h2>
              {repository.present && (
                <button
                  className="button"
                  ref={inspectorButton}
                  disabled={locked}
                  onClick={() =>
                    void run("Inspeccionando Git…", async () =>
                      setInspector(await inventoryApi.inspect(repository.id)),
                    )
                  }
                >
                  Abrir Inspector
                </button>
              )}
            </div>
            <p className="panel-intro">{repository.path}</p>
            <p className="inventory-meta">
              {repository.present
                ? kinds[repository.kind]
                : "Ausente del último escaneo"}
            </p>
            {identitySelect(repository, "Excepción de identidad")}
            <RepositoryDiagnosis diagnosis={view.diagnosis} />
            {view.diagnosis.conflictFingerprint && (
              <div className="action-row preview-actions">
                {!view.diagnosis.kept && (
                  <button
                    className="button"
                    disabled={locked}
                    onClick={() => {
                      const fingerprint = view.diagnosis?.conflictFingerprint;
                      if (fingerprint)
                        void run("Registrando decisión…", async () => {
                          await inventoryApi.keep(repository.id, fingerprint);
                          await refresh(directory?.id, repository.id);
                          setNotice("Configuración actual conservada.");
                        });
                    }}
                  >
                    Conservar configuración actual
                  </button>
                )}
                {repository.kind === "ordinary" &&
                  [
                    view.diagnosis.observed?.name,
                    view.diagnosis.observed?.email,
                  ].some((field) => field?.scope === "local") && (
                    <button
                      className="button"
                      ref={localReviewButton}
                      disabled={locked}
                      onClick={() =>
                        void run("Comprobando el archivo local…", async () =>
                          setLocalPreview(
                            await inventoryApi.previewLocal(repository.id),
                          ),
                        )
                      }
                    >
                      Revisar cambio local
                    </button>
                  )}
              </div>
            )}
            <details className="inventory-disclosure">
              <summary>Rutas de Git</summary>
              <dl className="observed-identity">
                <div>
                  <dt className="inventory-meta">Git dir</dt>
                  <dd>{repository.gitDir}</dd>
                </div>
                <div>
                  <dt className="inventory-meta">Git dir común</dt>
                  <dd>{repository.commonDir}</dd>
                </div>
              </dl>
            </details>
          </>
        ) : directory && view ? (
          <>
            <div className="detail-heading">
              <h2>{folderName(directory.path)}</h2>
              <div className="action-row">
                <button
                  className="button"
                  type="button"
                  ref={preferencesButton}
                  disabled={locked}
                  onClick={openScanPreferences}
                >
                  Preferencias
                </button>
                <button
                  className="button primary"
                  type="button"
                  ref={scanButton}
                  disabled={locked}
                  onClick={() => scan(directory)}
                >
                  Escanear
                </button>
              </div>
            </div>
            <p className="panel-intro">{directory.path}</p>
            {activeScan && (
              <section
                className="inventory-scan-progress"
                aria-labelledby="scan-progress-title"
              >
                <div className="inventory-scan-progress-heading">
                  <h3 id="scan-progress-title">
                    <span role="status">{scanStatus(activeScan)}</span>
                  </h3>
                  <button
                    className="button"
                    type="button"
                    ref={cancelScanButton}
                    disabled={!canCancelScan}
                    onClick={() => void cancelScan()}
                  >
                    Cancelar escaneo
                  </button>
                </div>
                {activeScan.progress && (
                  <dl className="inventory-scan-counts">
                    <div>
                      <dt>Entradas recorridas</dt>
                      <dd>{activeScan.progress.visited.toLocaleString()}</dd>
                    </div>
                    <div>
                      <dt>Repositorios verificados</dt>
                      <dd>{activeScan.progress.found.toLocaleString()}</dd>
                    </div>
                    <div>
                      <dt>Rutas sin verificar</dt>
                      <dd>{activeScan.progress.skipped.toLocaleString()}</dd>
                    </div>
                  </dl>
                )}
              </section>
            )}
            {identitySelect(directory, "Identidad del directorio")}
            <p className="inventory-meta">
              La regla define el perfil esperado. Revisa su aplicación para
              actualizar también los archivos de Git.
            </p>
            <div className="inventory-section">
              <h3>
                Repositorios <span>{view.contents.repositories.length}</span>
              </h3>
              {directory.scanPartial !== false && (
                <p className="inventory-meta">
                  {directory.scanPartial
                    ? "Escaneo parcial. El inventario puede incluir resultados anteriores."
                    : "Escanea para verificar qué repositorios hay bajo esta ruta."}
                </p>
              )}
              {view.contents.repositories.length > 0 ? (
                <ul className="inventory-list">
                  {view.contents.repositories.map((repo) => (
                    <li key={repo.id}>
                      <button
                        className="profile-choice"
                        disabled={locked}
                        onClick={() =>
                          void run("Consultando Git…", () =>
                            refresh(directory.id, repo.id),
                          )
                        }
                      >
                        <span className="profile-label">
                          {folderName(repo.path)}
                        </span>
                        <span className="profile-meta">{repo.path}</span>
                        <span className="profile-meta">
                          {repo.present
                            ? kinds[repo.kind]
                            : "Ausente del último escaneo"}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                directory.scanPartial === false && (
                  <p className="inventory-meta">
                    GitPilot no encontró repositorios en este directorio.
                  </p>
                )
              )}
            </div>
            <div className="inventory-section">
              <div className="detail-heading">
                <h3>Exclusiones</h3>
                <button
                  className="button"
                  disabled={locked}
                  onClick={() => openPathForm("exclude")}
                >
                  Excluir subcarpeta
                </button>
              </div>
              {view.contents.exclusions.length ? (
                <ul className="inventory-exclusions">
                  {view.contents.exclusions.map((excluded) => (
                    <li key={excluded}>
                      <span>{excluded}</span>
                      <button
                        className="button"
                        disabled={locked}
                        onClick={() =>
                          void run("Quitando exclusión…", async () => {
                            await inventoryApi.exclude(
                              directory,
                              excluded,
                              false,
                            );
                            await refresh();
                            setNotice(
                              "Exclusión retirada. Vuelve a escanear para actualizar la lista.",
                            );
                          })
                        }
                      >
                        Quitar exclusión
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="inventory-meta">Sin subcarpetas excluidas.</p>
              )}
            </div>
            <button
              className="button danger"
              disabled={locked}
              onClick={() => setMode("remove")}
            >
              Retirar directorio
            </button>
          </>
        ) : view ? (
          <div className="empty-identities">
            <h2>Registra tu primer directorio</h2>
            <p className="panel-intro">
              Selecciona una carpeta de trabajo para descubrir los repositorios
              que contiene y decidir qué perfil corresponde a cada uno.
            </p>
          </div>
        ) : (
          !error && <p role="status">Cargando directorios…</p>
        )}
      </section>
    </main>
  );
}
