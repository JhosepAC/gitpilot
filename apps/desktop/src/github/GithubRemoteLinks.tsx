import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  folderName,
  inventoryApi,
  inventoryError,
  type Directory,
  type Repository,
} from "../inventory/inventoryApi";
import {
  describeGithubProblem,
  type RepositoryScope,
} from "./githubResourceUi";

type Loaded<T> = { data: T } | { problem: unknown };
type Remote = {
  name: string;
  destination: string | null;
  fingerprint: string | null;
  version: string | null;
  state: "available" | "linked" | "review" | "other";
};
type Review = { action: "link" | "unlink"; remote: Remote };

const remoteStates = {
  available: "Disponible",
  linked: "Vinculado",
  review: "Requiere revisión",
  other: "Vinculado a otra selección",
};

function repositoryLabel(repository: Repository, root: Directory) {
  const path = repository.path.replaceAll("\\", "/");
  const parent = root.path.replaceAll("\\", "/").replace(/\/$/, "");
  return path.startsWith(`${parent}/`)
    ? path.slice(parent.length + 1)
    : folderName(path);
}

export function GithubRemoteLinks({
  scope,
  fullName,
}: {
  scope: RepositoryScope;
  fullName: string;
}) {
  const [directories, setDirectories] = useState<Loaded<Directory[]>>();
  const [directoryId, setDirectoryId] = useState<number>();
  const [catalog, setCatalog] = useState<
    { id: number } & Loaded<Repository[]>
  >();
  const [localId, setLocalId] = useState<number>();
  const roots = directories && "data" in directories ? directories.data : [];
  const root = roots.find((item) => item.id === directoryId) ?? roots[0];
  const selectedRoot = root?.id;
  const current = catalog?.id === selectedRoot ? catalog : undefined;
  const repositories = current && "data" in current ? current.data : [];
  const local = repositories.find((item) => item.id === localId);

  useEffect(() => {
    let active = true;
    void inventoryApi.directories().then(
      (data) => {
        if (active) setDirectories({ data });
      },
      (problem: unknown) => {
        if (active) setDirectories({ problem });
      },
    );
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (selectedRoot === undefined) return;
    let active = true;
    void inventoryApi.contents(selectedRoot).then(
      ({ repositories }) => {
        if (active)
          setCatalog({
            id: selectedRoot,
            data: repositories.filter((item) => item.present),
          });
      },
      (problem: unknown) => {
        if (active) setCatalog({ id: selectedRoot, problem });
      },
    );
    return () => {
      active = false;
    };
  }, [selectedRoot]);

  return (
    <section
      className="github-repository-content"
      aria-labelledby="github-link-title"
    >
      <h3 id="github-link-title">Vincular un remoto del inventario</h3>
      <p className="inventory-meta">
        Elige el repositorio local para revisar su destino de lectura.
      </p>
      {(!directories || (root && !current)) && (
        <p className="inventory-meta" role="status">
          Cargando inventario…
        </p>
      )}
      {directories && "problem" in directories && (
        <p className="inventory-conflict" role="alert">
          {inventoryError(directories.problem)}
        </p>
      )}
      {directories && "data" in directories && roots.length === 0 && (
        <p className="inventory-meta">
          Registra y escanea una carpeta desde Directorios para vincular un
          remoto.
        </p>
      )}
      {root && (
        <div className="github-local-picker">
          <label className="field">
            <span>Raíz del inventario</span>
            <select
              value={root.id}
              onChange={(event) => {
                setDirectoryId(Number(event.target.value));
                setLocalId(undefined);
              }}
            >
              {roots.map((item) => (
                <option key={item.id} value={item.id}>
                  {folderName(item.path)} · {item.path}
                </option>
              ))}
            </select>
          </label>
          {repositories.length > 0 && (
            <label className="field">
              <span>Repositorio local</span>
              <select
                value={local?.id ?? ""}
                onChange={(event) => {
                  setLocalId(
                    event.target.value ? Number(event.target.value) : undefined,
                  );
                }}
              >
                <option value="">Selecciona un repositorio</option>
                {repositories.map((item) => (
                  <option key={item.id} value={item.id}>
                    {repositoryLabel(item, root)}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
      )}
      {current && "problem" in current && (
        <p className="inventory-conflict" role="alert">
          {inventoryError(current.problem)}
        </p>
      )}
      {current && "data" in current && repositories.length === 0 && (
        <p className="inventory-meta">
          La raíz no tiene repositorios disponibles. Escanéala desde
          Directorios.
        </p>
      )}
      {local && (
        <GithubLocalRemotes
          key={local.id}
          local={local}
          target={{ scope, fullName }}
        />
      )}
    </section>
  );
}

export function GithubLocalRemotes({
  local,
  target,
}: {
  local: Pick<Repository, "id" | "path">;
  target?: { scope: RepositoryScope; fullName: string };
}) {
  const [request, setRequest] = useState(0);
  const [result, setResult] = useState<
    { request: number } & Loaded<{ remotes: Remote[]; partial: boolean }>
  >();
  const [review, setReview] = useState<Review>();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [problem, setProblem] = useState("");
  const cancel = useRef<HTMLButtonElement>(null);
  const remotesTitle = useRef<HTMLHeadingElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const current = result?.request === request ? result : undefined;
  const catalog = current && "data" in current ? current.data : undefined;
  const scope = target?.scope;

  useEffect(() => {
    let active = true;
    void invoke<{ remotes: Remote[]; partial: boolean }>(
      "github_remote_candidates",
      { localId: local.id, scope: scope ?? null },
    ).then(
      (data) => {
        if (active) setResult({ request, data });
      },
      (problem: unknown) => {
        if (active) setResult({ request, problem });
      },
    );
    return () => {
      active = false;
    };
  }, [local.id, request, scope]);

  useEffect(() => {
    if (review) cancel.current?.focus();
  }, [review]);

  async function confirm() {
    if (!review || busy) return;
    const { remote, action } = review;
    if (action === "link" && !scope) return;
    setBusy(true);
    setNotice("");
    setProblem("");
    try {
      if (action === "link") {
        await invoke("github_link_remote", {
          scope,
          review: {
            localId: local.id,
            remote: remote.name,
            fingerprint: remote.fingerprint,
            version: remote.version,
          },
        });
        setNotice("Vínculo guardado.");
      } else {
        await invoke("github_unlink_remote", {
          localId: local.id,
          remote: remote.name,
          version: remote.version,
        });
        setNotice("Vínculo retirado.");
      }
    } catch (failure) {
      setProblem(describeGithubProblem(failure));
    } finally {
      setReview(undefined);
      setBusy(false);
      setRequest((previous) => previous + 1);
      remotesTitle.current?.focus();
    }
  }

  return (
    <div className="github-local-remotes">
      <h3 ref={remotesTitle} tabIndex={-1}>
        {target ? "Remotos de lectura" : "Vínculos guardados"}
      </h3>
      {!current && (
        <p className="inventory-meta" role="status">
          Consultando Git…
        </p>
      )}
      {current && "problem" in current && (
        <p className="inventory-conflict" role="alert">
          {describeGithubProblem(current.problem)}
        </p>
      )}
      {catalog?.partial && (
        <p className="inventory-meta" role="status">
          La comprobación de remotos es parcial.
        </p>
      )}
      {catalog?.remotes.length === 0 && (
        <p className="inventory-meta">
          {target
            ? "No se encontraron remotos directos de GitHub.com."
            : "Este repositorio local no tiene vínculos guardados."}
        </p>
      )}
      {catalog && catalog.remotes.length > 0 && (
        <ul className="github-resource-list">
          {catalog.remotes.map((remote) => {
            const canLink =
              target &&
              remote.fingerprint &&
              remote.destination?.toLowerCase() ===
                target.fullName.toLowerCase();
            return (
              <li key={remote.name} className="github-resource-row">
                <div>
                  <strong>{remote.name}</strong>
                  <p className="inventory-meta">
                    {remote.destination ?? "Destino sin comprobar"} ·{" "}
                    {canLink || remote.version
                      ? remoteStates[remote.state]
                      : "Otro repositorio"}
                  </p>
                </div>
                <div className="github-inline-actions">
                  {canLink && remote.state !== "linked" && (
                    <button
                      className="button"
                      type="button"
                      disabled={Boolean(review)}
                      onClick={(event) => {
                        trigger.current = event.currentTarget;
                        setReview({ action: "link", remote });
                      }}
                    >
                      {remote.state === "available"
                        ? "Vincular"
                        : "Revisar vínculo"}
                    </button>
                  )}
                  {remote.version && (
                    <button
                      className="button danger"
                      type="button"
                      disabled={Boolean(review)}
                      onClick={(event) => {
                        trigger.current = event.currentTarget;
                        setReview({ action: "unlink", remote });
                      }}
                    >
                      Quitar vínculo
                    </button>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      )}
      {review && (
        <div className="github-link-review">
          <h3>
            {review.action === "link" ? "Revisar vínculo" : "Revisar retirada"}
          </h3>
          <p className="inventory-meta">{local.path}</p>
          <p>
            {review.action === "link" ? (
              <>
                Vincular <strong>{review.remote.name}</strong> con{" "}
                <strong>{target?.fullName}</strong>.
              </>
            ) : (
              <>
                Quitar la asociación guardada del remoto{" "}
                <strong>{review.remote.name}</strong>.
              </>
            )}
          </p>
          {review.action === "link" && review.remote.state === "other" && (
            <p className="inventory-meta">
              Se sustituirá la asociación anterior de este remoto.
            </p>
          )}
          <div className="action-row">
            <button
              className={
                review.action === "link" ? "button primary" : "button danger"
              }
              type="button"
              disabled={busy}
              onClick={() => void confirm()}
            >
              {review.action === "link"
                ? "Confirmar vínculo"
                : "Confirmar retirada"}
            </button>
            <button
              className="button"
              type="button"
              ref={cancel}
              disabled={busy}
              onClick={() => {
                setReview(undefined);
                requestAnimationFrame(() => trigger.current?.focus());
              }}
            >
              Cancelar
            </button>
          </div>
        </div>
      )}
      {current && !review && (
        <div className="action-row">
          <button
            className="button"
            type="button"
            onClick={() => {
              setProblem("");
              setNotice("");
              setRequest((previous) => previous + 1);
              remotesTitle.current?.focus();
            }}
          >
            Actualizar remotos
          </button>
        </div>
      )}
      {busy && (
        <p className="inventory-meta" role="status">
          Comprobando el vínculo…
        </p>
      )}
      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}
      {problem && (
        <p className="inventory-conflict" role="alert">
          {problem}
        </p>
      )}
    </div>
  );
}
