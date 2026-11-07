import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Brand } from "./Brand";
import { WorkspaceNav } from "./WorkspaceNav";
import type { Workspace } from "./WorkspaceNav";
import { NativeOperationPanel } from "../inventory/ConfigPreview";
import { inventoryApi, inventoryError } from "../inventory/inventoryApi";
import type { ActivityItem, NativeOperation } from "../inventory/inventoryApi";
import "../inventory/inventory.css";
import "./overview.css";

type Navigate = (workspace: Workspace) => void;

function Overview({
  workspace,
  title,
  subtitle,
  busy,
  onNavigate,
  children,
}: {
  workspace: "setup" | "activity";
  title: string;
  subtitle: string;
  busy?: boolean;
  onNavigate: Navigate;
  children: ReactNode;
}) {
  return (
    <main className="identity-shell">
      <header className="identity-heading">
        <div>
          <h1>{title}</h1>
          <p>{subtitle}</p>
        </div>
      </header>
      <aside className="identity-sidebar" aria-label="Espacio de trabajo">
        <Brand />
        <WorkspaceNav
          current={workspace}
          onNavigate={onNavigate}
          disabled={busy}
        />
        <footer className="sidebar-foot">Almacenamiento local</footer>
      </aside>
      <section
        className="identity-panel inventory-panel overview-panel"
        aria-label={title}
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
      title="Inicio"
      subtitle="Prepara tus identidades para Git."
      onNavigate={onNavigate}
    >
      <h2>Configura tu espacio de trabajo</h2>
      <p className="panel-intro">
        Guarda tus perfiles, elige dónde usarlos y revisa la propuesta antes de
        cambiar la configuración de Git.
      </p>
      {error ? (
        <p role="alert">{error}</p>
      ) : !status ? (
        <p role="status">Comprobando Git…</p>
      ) : (
        <ol className="overview-list">
          <li>
            <div>
              <h3>Comprobar Git</h3>
              <p className="inventory-meta">
                {status.compatibleGit
                  ? status.gitVersion
                  : status.gitVersion
                    ? `${status.gitVersion} · Actualiza a Git 2.39.5 o posterior.`
                    : "Instala Git 2.39.5 o posterior, agrégalo a PATH y vuelve a abrir GitPilot."}
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
              Comprobar de nuevo
            </button>
          </li>
          <li>
            <div>
              <h3>Crear o importar un perfil</h3>
              <p className="inventory-meta">
                {status.identities
                  ? `Perfiles guardados · ${status.identities}`
                  : "Define el nombre y el correo del commit."}
              </p>
            </div>
            <button
              className={status.identities ? "button" : "button primary"}
              onClick={() => onNavigate("identities")}
            >
              Ir a identidades
            </button>
          </li>
          <li>
            <div>
              <h3>Registrar y asignar un directorio</h3>
              <p className="inventory-meta">
                {status.assignedDirectories
                  ? `Directorios con regla propia · ${status.assignedDirectories}`
                  : "Escanea una carpeta de trabajo y elige su perfil."}
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
              Ir a directorios
            </button>
          </li>
          <li>
            <div>
              <h3>Revisar la configuración</h3>
              <p className="inventory-meta">
                {status.applied
                  ? "Hay una aplicación global verificada."
                  : "Abre Vista previa de Git en Directorios para comprobar el alcance."}
              </p>
            </div>
          </li>
        </ol>
      )}
    </Overview>
  );
}

const stateLabel: Record<ActivityItem["state"], string> = {
  preparing: "Aplicación pendiente",
  applied: "Aplicada",
  restoring: "Restauración pendiente",
  restored: "Restaurada",
  rolled_back: "Aplicación revertida",
  kept: "Configuración conservada",
};

const timestamp = new Intl.DateTimeFormat("es", {
  dateStyle: "medium",
  timeStyle: "short",
});

export function ActivityPage({ onNavigate }: { onNavigate: Navigate }) {
  const [items, setItems] = useState<ActivityItem[]>();
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
      title="Actividad"
      subtitle="Aplicaciones, restauraciones y decisiones."
      busy={busy}
      onNavigate={onNavigate}
    >
      <div className="identity-feedback">
        <div role="status">{busy ? "Comprobando operación…" : notice}</div>
        {error && <p role="alert">{error}</p>}
      </div>
      {selected ? (
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
                setNotice("Configuración anterior restaurada y verificada.");
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
      ) : (
        <>
          <h2>Operaciones recientes</h2>
          {items?.length ? (
            <ul className="overview-list">
              {items.map((item) => (
                <li key={item.id}>
                  <div>
                    <h3>
                      {stateLabel[item.state]}{" "}
                      <span className="inventory-meta">
                        ·{" "}
                        {item.scope === "global"
                          ? "Global"
                          : item.scope === "hook"
                            ? "Pre-commit"
                            : "Repositorio"}
                      </span>
                    </h3>
                    <p>{item.target}</p>
                    <time className="inventory-meta">
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
                      Revisar operación
                    </button>
                  )}
                </li>
              ))}
            </ul>
          ) : (
            !error && (
              <p className="panel-intro" role="status">
                {items
                  ? "Las operaciones que confirmes aparecerán aquí."
                  : "Cargando actividad…"}
              </p>
            )
          )}
        </>
      )}
    </Overview>
  );
}
