import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Brand } from "./Brand";
import { WorkspaceNav } from "./WorkspaceNav";
import type { Workspace } from "./WorkspaceNav";
import { useLanguage } from "./language";
import { NativeOperationPanel } from "../inventory/ConfigPreview";
import { inventoryApi, inventoryError } from "../inventory/inventoryApi";
import type { ActivityItem, NativeOperation } from "../inventory/inventoryApi";
import "../inventory/inventory.css";
import "./overview.css";

type Navigate = (workspace: Workspace) => void;

export function Overview({
  workspace,
  title,
  subtitle,
  busy,
  contentLanguage,
  onNavigate,
  children,
}: {
  workspace: "setup" | "activity" | "github" | "settings";
  title: string;
  subtitle: string;
  busy?: boolean;
  contentLanguage?: "es";
  onNavigate: Navigate;
  children: ReactNode;
}) {
  const { t } = useLanguage();
  return (
    <main className="identity-shell">
      <header className="identity-heading">
        <div>
          <h1 id="workspace-title" tabIndex={-1}>
            {title}
          </h1>
          <p>{subtitle}</p>
        </div>
      </header>
      <aside
        className="identity-sidebar"
        aria-label={t("Espacio de trabajo", "Workspace")}
      >
        <Brand />
        <WorkspaceNav
          current={workspace}
          onNavigate={onNavigate}
          disabled={busy}
        />
        <footer className="sidebar-foot">
          {t("Almacenamiento local", "Local storage")}
        </footer>
      </aside>
      <section
        className="identity-panel inventory-panel overview-panel"
        aria-label={title}
        lang={contentLanguage}
      >
        {children}
      </section>
    </main>
  );
}

export type SetupStatus = {
  gitVersion: string | null;
  compatibleGit: boolean;
  identities: number;
  directories: number;
  assignedDirectories: number;
  applied: boolean;
};

export const loadSetup = () => invoke<SetupStatus>("setup_status");

export function SetupPage({ onNavigate }: { onNavigate: Navigate }) {
  const { t } = useLanguage();
  const [status, setStatus] = useState<SetupStatus>();
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let mounted = true;
    void loadSetup().then(
      (data) => {
        if (mounted) setStatus(data);
      },
      (problem) => {
        if (mounted) setError(inventoryError(problem));
      },
    );
    return () => {
      mounted = false;
    };
  }, [revision]);

  return (
    <Overview
      workspace="setup"
      title={t("Inicio", "Home")}
      subtitle={t(
        "Prepara tus identidades para Git.",
        "Set up your identities for Git.",
      )}
      onNavigate={onNavigate}
    >
      <h2>{t("Configura tu espacio de trabajo", "Set up your workspace")}</h2>
      <p className="panel-intro">
        {t(
          "Guarda tus perfiles, elige dónde usarlos y revisa la propuesta antes de cambiar la configuración de Git.",
          "Save your profiles, choose where to use them, and review the proposal before changing Git configuration.",
        )}
      </p>
      {error ? (
        <p role="alert" lang="es">
          {error}
        </p>
      ) : !status ? (
        <p role="status">{t("Comprobando Git…", "Checking Git…")}</p>
      ) : (
        <ol className="overview-list">
          <li>
            <div>
              <h3>{t("Comprobar Git", "Check Git")}</h3>
              <p className="inventory-meta">
                {status.compatibleGit
                  ? status.gitVersion
                  : status.gitVersion
                    ? `${status.gitVersion} · ${t("Actualiza a Git 2.39.5 o posterior.", "Update to Git 2.39.5 or later.")}`
                    : t(
                        "Instala Git 2.39.5 o posterior, agrégalo a PATH y vuelve a abrir GitPilot.",
                        "Install Git 2.39.5 or later, add it to PATH, and reopen GitPilot.",
                      )}
              </p>
            </div>
            <button
              className="button"
              onClick={() => {
                setStatus(undefined);
                setError("");
                setRevision(revision + 1);
              }}
            >
              {t("Comprobar de nuevo", "Check again")}
            </button>
          </li>
          <li>
            <div>
              <h3>
                {t("Crear o importar un perfil", "Create or import a profile")}
              </h3>
              <p className="inventory-meta">
                {status.identities
                  ? `${t("Perfiles guardados", "Saved profiles")} · ${status.identities}`
                  : t(
                      "Define el nombre y el correo del commit.",
                      "Set the commit author name and email.",
                    )}
              </p>
            </div>
            <button
              className={status.identities ? "button" : "button primary"}
              onClick={() => onNavigate("identities")}
            >
              {t("Ir a identidades", "Open identities")}
            </button>
          </li>
          <li>
            <div>
              <h3>
                {t(
                  "Registrar y asignar un directorio",
                  "Register and assign a directory",
                )}
              </h3>
              <p className="inventory-meta">
                {status.assignedDirectories
                  ? `${t("Directorios con regla propia", "Directories with assignments")} · ${status.assignedDirectories}`
                  : t(
                      "Escanea una carpeta de trabajo y elige su perfil.",
                      "Scan a work folder and choose its profile.",
                    )}
              </p>
            </div>
            <button
              className={
                status.identities && !status.directories
                  ? "button primary"
                  : "button"
              }
              onClick={() => onNavigate("directories")}
            >
              {t("Ir a directorios", "Open directories")}
            </button>
          </li>
          <li>
            <div>
              <h3>{t("Revisar la configuración", "Review configuration")}</h3>
              <p className="inventory-meta">
                {status.applied
                  ? t(
                      "Hay una aplicación global verificada.",
                      "A global configuration has been applied and verified.",
                    )
                  : t(
                      "Abre Vista previa de Git en Directorios para comprobar el alcance.",
                      "Open the Git preview in Directories to check its scope.",
                    )}
              </p>
            </div>
          </li>
        </ol>
      )}
    </Overview>
  );
}

export function ActivityPage({ onNavigate }: { onNavigate: Navigate }) {
  const { language, t } = useLanguage();
  const stateLabel: Record<ActivityItem["state"], string> = {
    preparing: t("Aplicación pendiente", "Application pending"),
    applied: t("Aplicada", "Applied"),
    restoring: t("Restauración pendiente", "Restore pending"),
    restored: t("Restaurada", "Restored"),
    rolled_back: t("Aplicación revertida", "Rolled back"),
    kept: t("Configuración conservada", "Configuration kept"),
    exported: t("Configuración exportada", "Settings exported"),
    imported: t("Configuración importada", "Settings imported"),
  };
  const scopeLabel: Record<ActivityItem["scope"], string> = {
    global: "Global",
    hook: "Pre-commit",
    local: t("Repositorio", "Repository"),
    data: t("Portabilidad", "Portability"),
  };
  const timestamp = new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  });
  const [items, setItems] = useState<ActivityItem[]>();
  const [filter, setFilter] = useState<"all" | "git" | "data">("all");
  const visible = items?.filter(
    (item) =>
      filter === "all" ||
      (filter === "data" ? item.scope === "data" : item.scope !== "data"),
  );
  const [selected, setSelected] = useState<NativeOperation>();
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let mounted = true;
    void inventoryApi.activity().then(
      (data) => {
        if (mounted) setItems(data);
      },
      (problem) => {
        if (mounted) setError(inventoryError(problem));
      },
    );
    return () => {
      mounted = false;
    };
  }, []);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (problem) {
      setError(inventoryError(problem));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Overview
      workspace="activity"
      title={t("Actividad", "Activity")}
      subtitle={t(
        "Historial local de Git y transferencias de datos.",
        "Local history of Git operations and data transfers.",
      )}
      busy={busy}
      onNavigate={onNavigate}
    >
      <div className="identity-feedback">
        <div role="status">
          {busy ? t("Comprobando operación…", "Checking operation…") : notice}
        </div>
        {error && (
          <p role="alert" lang="es">
            {error}
          </p>
        )}
      </div>
      {selected ? (
        <div lang="es">
          <NativeOperationPanel
            operation={selected}
            busy={busy}
            backLabel="Volver a actividad"
            onBack={() => {
              setSelected(undefined);
              requestAnimationFrame(() =>
                document.getElementById(`activity-${selected.id}`)?.focus(),
              );
            }}
            onRestore={(confirmation) =>
              void run(async () => {
                try {
                  await inventoryApi.confirmNative(confirmation, true, {
                    operationId: selected.id,
                  });
                  setSelected(undefined);
                  setNotice(
                    t(
                      "Configuración anterior restaurada y verificada.",
                      "Previous configuration restored and verified.",
                    ),
                  );
                } catch (problem) {
                  setSelected(
                    (await inventoryApi.nativeStatus(selected.id)) ?? undefined,
                  );
                  throw problem;
                } finally {
                  setItems(await inventoryApi.activity());
                }
              })
            }
          />
        </div>
      ) : (
        <>
          <h2>{t("Operaciones recientes", "Recent operations")}</h2>
          <div className="settings-row activity-controls">
            <label htmlFor="activity-filter">{t("Mostrar", "Show")}</label>
            <select
              id="activity-filter"
              value={filter}
              onChange={(event) =>
                setFilter(event.currentTarget.value as typeof filter)
              }
            >
              <option value="all">{t("Todas", "All")}</option>
              <option value="git">Git</option>
              <option value="data">{scopeLabel.data}</option>
            </select>
            <button
              className="button"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  setItems(await inventoryApi.activity());
                  setNotice(t("Actividad actualizada.", "Activity refreshed."));
                })
              }
            >
              {t("Actualizar", "Refresh")}
            </button>
          </div>
          {visible?.length ? (
            <ul className="overview-list">
              {visible.map((item) => (
                <li key={`${item.scope}:${item.id}`}>
                  <div>
                    <h3>
                      {stateLabel[item.state]}{" "}
                      <span className="inventory-meta">
                        · {scopeLabel[item.scope]}
                      </span>
                    </h3>
                    <p>
                      {item.profileCount === null
                        ? item.target
                        : t(
                            `Perfiles en el archivo · ${item.profileCount}`,
                            `Profiles in file · ${item.profileCount}`,
                          )}
                    </p>
                    <time
                      className="inventory-meta"
                      dateTime={item.at.replace(" ", "T") + "Z"}
                    >
                      {timestamp.format(
                        new Date(item.at.replace(" ", "T") + "Z"),
                      )}
                    </time>
                  </div>
                  {["applied", "preparing", "restoring"].includes(
                    item.state,
                  ) && (
                    <button
                      className="button"
                      id={`activity-${item.id}`}
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          const status = await inventoryApi.nativeStatus(
                            item.id,
                          );
                          if (status) setSelected(status);
                          else setItems(await inventoryApi.activity());
                        })
                      }
                    >
                      {t("Revisar operación", "Review operation")}
                    </button>
                  )}
                </li>
              ))}
            </ul>
          ) : (
            !error && (
              <p className="panel-intro" role="status">
                {items
                  ? filter === "all"
                    ? t(
                        "Las operaciones que confirmes aparecerán aquí.",
                        "Confirmed operations will appear here.",
                      )
                    : t(
                        "No hay operaciones para este filtro.",
                        "No operations match this filter.",
                      )
                  : t("Cargando actividad…", "Loading activity…")}
              </p>
            )
          )}
        </>
      )}
    </Overview>
  );
}
