import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Brand } from "../app/Brand";
import { WorkspaceNav } from "../app/WorkspaceNav";
import { ConfigPreview } from "./ConfigPreview";
import { RepositoryDiagnosis } from "./RepositoryDiagnosis";
import {
  folderName,
  inventoryApi,
  inventoryError,
  loadInventory,
} from "./inventoryApi";
import type { Directory, NativePreview, Repository } from "./inventoryApi";
import "./inventory.css";

const kinds = {
  ordinary: "Repositorio",
  worktree: "Worktree vinculado",
  separate: "Submódulo o Git dir separado",
  bare: "Repositorio bare",
};

export function InventoryPage({
  onNavigate,
}: {
  onNavigate: (workspace: "identities" | "directories") => void;
}) {
  const [view, setView] = useState<Awaited<ReturnType<typeof loadInventory>>>();
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [mode, setMode] = useState<"add" | "exclude" | "remove" | null>(null);
  const [preview, setPreview] = useState<NativePreview>();
  const [path, setPath] = useState("");
  const panel = useRef<HTMLElement>(null);
  const pathInput = useRef<HTMLInputElement>(null);
  const previewButton = useRef<HTMLButtonElement>(null);
  const directory = view?.directory;
  const repository = view?.repository;
  const locked = Boolean(busy || mode || preview);

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
  }, [directory?.id, repository?.id, mode, preview]);

  useEffect(() => {
    pathInput.current?.focus();
  }, [mode]);

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
    void run("Escaneando repositorios…", async () => {
      const report = await inventoryApi.scan(root.id);
      await refresh(root.id);
      setNotice(
        report.partial
          ? `Escaneo parcial. Repositorios verificados: ${report.found}. Rutas sin verificar: ${report.skipped}.`
          : `Escaneo completo. Repositorios encontrados: ${report.found}.`,
      );
    });
  }

  function assign(target: Directory | Repository, value: string) {
    void run("Guardando asignación…", async () => {
      await inventoryApi.assign(
        target,
        "kind" in target,
        value ? Number(value) : null,
      );
      await refresh(directory?.id, repository?.id);
      setNotice("Asignación guardada en GitPilot.");
    });
  }

  function identitySelect(target: Directory | Repository, label: string) {
    return (
      <label className="field inventory-select">
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
    );
  }

  return (
    <main className="identity-shell">
      <header className="identity-heading">
        <div>
          <h1>Directorios</h1>
          <p>Repositorios y reglas de identidad locales.</p>
        </div>
        <div className="action-row">
          {directory && !mode && !preview && (
            <button
              className="button"
              ref={previewButton}
              disabled={locked}
              onClick={() =>
                void run("Simulando configuración…", async () => {
                  setPreview(await inventoryApi.preview());
                })
              }
            >
              Vista previa de Git
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
              Actualizar vista
            </button>
          )}
          {view && !mode && !preview && (
            <button
              className={directory ? "button" : "button primary"}
              disabled={locked}
              onClick={() => openPathForm("add")}
            >
              Añadir directorio
            </button>
          )}
        </div>
      </header>
      <aside className="identity-sidebar" aria-label="Directorios registrados">
        <Brand />
        <WorkspaceNav
          current="directories"
          onNavigate={onNavigate}
          disabled={locked}
        />
        <div className="list-heading">
          <h2>Directorios</h2>
          {view && <span>{view.directories.length}</span>}
        </div>
        {view?.directories.length ? (
          <ul className="profile-list">
            {view.directories.map((root) => (
              <li key={root.id}>
                <button
                  type="button"
                  className="profile-choice"
                  aria-current={root.id === directory?.id ? "true" : undefined}
                  disabled={locked}
                  onClick={() =>
                    void run("Cargando directorio…", () => refresh(root.id))
                  }
                >
                  <span className="profile-label">{folderName(root.path)}</span>
                  <span className="profile-meta">{root.path}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="profile-empty">
            {view ? "Aún no hay directorios." : "Inventario local"}
          </p>
        )}
        <footer className="sidebar-foot">Almacenamiento local</footer>
      </aside>
      <section
        className="identity-panel inventory-panel"
        aria-label="Inventario"
        ref={panel}
      >
        <div className="identity-feedback">
          <div role="status">{busy || notice}</div>
          {error && <p role="alert">{error}</p>}
        </div>
        {preview ? (
          <ConfigPreview
            preview={preview}
            onBack={() => {
              setPreview(undefined);
              requestAnimationFrame(() => previewButton.current?.focus());
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
              <label className="field">
                Ruta absoluta
                <input
                  ref={pathInput}
                  value={path}
                  onChange={(event) => setPath(event.currentTarget.value)}
                  placeholder={mode === "exclude" ? directory?.path : undefined}
                  required
                  disabled={Boolean(busy)}
                  spellCheck={false}
                />
              </label>
              <div className="action-row">
                <button className="button primary" disabled={Boolean(busy)}>
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
            <h2>{folderName(repository.path)}</h2>
            <p className="panel-intro">{repository.path}</p>
            <p className="inventory-meta">
              {repository.present
                ? kinds[repository.kind]
                : "Ausente del último escaneo"}
            </p>
            {identitySelect(repository, "Excepción de identidad")}
            <RepositoryDiagnosis diagnosis={view.diagnosis} />
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
              <button
                className="button primary"
                disabled={locked}
                onClick={() => scan(directory)}
              >
                Escanear
              </button>
            </div>
            <p className="panel-intro">{directory.path}</p>
            {identitySelect(directory, "Identidad del directorio")}
            <p className="inventory-meta">
              Esta regla se guarda en GitPilot para calcular la identidad
              esperada.
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
