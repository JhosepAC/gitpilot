import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

type Installation = {
  id: number;
  login: string;
  kind: "User" | "Organization" | "Other";
  selection: "all" | "selected" | "unknown";
  suspended: boolean;
};

type Repository = {
  id: number;
  fullName: string;
  visibility: "public" | "private" | "internal";
  archived: boolean;
};

type Page<T> = { items: T[]; hasNext: boolean; partialSso: boolean };
type Query = { installation: Installation | null; page: number };
type Result = { query: Query } & (
  | { installations: Page<Installation> }
  | { repositories: Page<Repository> }
  | { problem: unknown }
);

const visibility = {
  public: "Público",
  private: "Privado",
  internal: "Interno",
};
const ownerKind = {
  User: "Cuenta personal",
  Organization: "Organización",
  Other: "Instalación",
};
const selection = {
  all: "Todos los repositorios",
  selected: "Repositorios seleccionados",
  unknown: "Acceso según GitHub",
};

const problems: Record<string, string> = {
  permission_denied:
    "GitHub denegó el acceso. Revisa los permisos de la instalación.",
  installation_unavailable:
    "La instalación dejó de estar disponible o esta cuenta perdió acceso.",
  sso_required:
    "GitHub requiere que inicies sesión con el SSO de la organización. Completa el acceso en GitHub y reintenta.",
  expired: "La sesión venció. Vuelve a la cuenta para autorizarla de nuevo.",
  not_connected:
    "GitHub ya no acepta esta sesión. Vuelve a la cuenta para autorizarla.",
  renewal_uncertain:
    "No se pudo comprobar la renovación. Vuelve a la cuenta para revisar la sesión.",
  secure_store: "Desbloquea el almacén seguro del sistema y reintenta.",
  network:
    "No se pudo contactar con GitHub. Comprueba la conexión y reintenta.",
  busy: "Hay otra operación de GitHub en curso. Espera y reintenta.",
  rate_limited:
    "GitHub limitó la consulta. Espera al menos un minuto antes de reintentar.",
};

function describe(problem: unknown) {
  if (
    typeof problem === "object" &&
    problem !== null &&
    "retryAt" in problem &&
    typeof problem.retryAt === "number"
  ) {
    const date = new Date(problem.retryAt * 1000);
    if (Number.isFinite(date.getTime())) {
      return `GitHub limitó las consultas. Reintenta después de ${date.toLocaleString()}.`;
    }
    return "GitHub limitó las consultas. Espera antes de reintentar.";
  }
  return (
    (typeof problem === "string" && problems[problem]) ||
    "No se pudo consultar GitHub. Vuelve a intentarlo."
  );
}

export function GithubResources({
  githubId,
  login,
  onBack,
}: {
  githubId: number;
  login: string;
  onBack: () => void;
}) {
  const [query, setQuery] = useState<Query>({ installation: null, page: 1 });
  const [result, setResult] = useState<Result>();
  const installationPage = useRef(1);
  const heading = useRef<HTMLHeadingElement>(null);
  const current = result?.query === query ? result : undefined;
  const installations =
    current && "installations" in current ? current.installations : undefined;
  const repositories =
    current && "repositories" in current ? current.repositories : undefined;
  const page = installations ?? repositories;

  useEffect(() => {
    let active = true;
    const args = { githubId, page: query.page };
    const request = query.installation
      ? invoke<Page<Repository>>("github_repositories", {
          ...args,
          installationId: query.installation.id,
        }).then((repositories) => ({ query, repositories }))
      : invoke<Page<Installation>>("github_installations", args).then(
          (installations) => ({ query, installations }),
        );
    void request.then(
      (response) => {
        if (active) setResult(response);
      },
      (problem: unknown) => {
        if (active) setResult({ query, problem });
      },
    );
    heading.current?.focus();
    return () => {
      active = false;
    };
  }, [githubId, query]);

  function back() {
    if (query.installation) {
      setQuery({ installation: null, page: installationPage.current });
    } else {
      onBack();
    }
  }

  return (
    <section
      className="github-resources"
      aria-labelledby="github-resource-title"
    >
      <div className="github-resource-heading">
        <div>
          <h2 id="github-resource-title" tabIndex={-1} ref={heading}>
            {query.installation ? "Repositorios" : "Instalaciones"}
          </h2>
          <p className="inventory-meta">
            {query.installation?.login ?? `@${login}`}
          </p>
        </div>
        <button className="button" type="button" onClick={back}>
          {query.installation ? "Volver a instalaciones" : "Volver a la cuenta"}
        </button>
      </div>
      {!current && (
        <p role="status" className="inventory-meta">
          Consultando GitHub…
        </p>
      )}
      {current && "problem" in current && (
        <p role="alert" className="inventory-conflict">
          {describe(current.problem)}
        </p>
      )}
      {page?.partialSso && (
        <p role="status" className="inventory-meta">
          GitHub devolvió resultados parciales por el SSO de una organización.
        </p>
      )}
      {page?.items.length === 0 && query.page > 1 && (
        <p className="panel-intro">Esta página ya no tiene resultados.</p>
      )}
      {query.page === 1 && installations?.items.length === 0 && (
        <p className="panel-intro">
          Esta cuenta no tiene instalaciones visibles de GitPilot. Revisa en
          GitHub que la aplicación esté instalada y aprobada.
        </p>
      )}
      {query.page === 1 && repositories?.items.length === 0 && (
        <p className="panel-intro">
          Esta cuenta no tiene repositorios accesibles en la instalación. Revisa
          qué repositorios tiene autorizados GitPilot.
        </p>
      )}
      {installations && installations.items.length > 0 && (
        <ul className="github-resource-list">
          {installations.items.map((item) => (
            <li key={item.id} className="github-resource-row">
              <div>
                <strong>{item.login}</strong>
                <p className="inventory-meta">
                  {ownerKind[item.kind]} ·{" "}
                  {item.suspended ? "Suspendida" : selection[item.selection]}
                </p>
              </div>
              {!item.suspended && (
                <button
                  className="button"
                  type="button"
                  aria-label={`Ver repositorios de ${item.login}`}
                  onClick={() => {
                    installationPage.current = query.page;
                    setQuery({ installation: item, page: 1 });
                  }}
                >
                  Ver repositorios
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {repositories && repositories.items.length > 0 && (
        <ul className="github-resource-list">
          {repositories.items.map((item) => (
            <li key={item.id} className="github-resource-row">
              <strong>{item.fullName}</strong>
              <span className="inventory-meta">
                {visibility[item.visibility]}
                {item.archived ? " · Archivado" : ""}
              </span>
            </li>
          ))}
        </ul>
      )}
      {current && (
        <div className="action-row">
          {(query.page > 1 || page?.hasNext) && (
            <nav
              className="github-pagination"
              aria-label="Páginas de resultados"
            >
              <button
                className="button"
                type="button"
                disabled={query.page === 1}
                onClick={() => setQuery({ ...query, page: query.page - 1 })}
              >
                Anterior
              </button>
              <span className="inventory-meta">Página {query.page}</span>
              <button
                className="button"
                type="button"
                disabled={!page?.hasNext}
                onClick={() => setQuery({ ...query, page: query.page + 1 })}
              >
                Siguiente
              </button>
            </nav>
          )}
          <button
            className="button"
            type="button"
            onClick={() => setQuery({ ...query })}
          >
            {"problem" in current ? "Reintentar" : "Actualizar"}
          </button>
        </div>
      )}
    </section>
  );
}
