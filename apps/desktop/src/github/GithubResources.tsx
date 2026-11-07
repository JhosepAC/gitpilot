import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { GithubRepository } from "./GithubRepository";
import {
  describeGithubProblem,
  GithubPageActions,
  visibility,
  type GithubRepositoryItem,
  type Page,
  type RepositoryScope,
} from "./githubResourceUi";

type Installation = {
  id: number;
  login: string;
  kind: "User" | "Organization" | "Other";
  selection: "all" | "selected" | "unknown";
  suspended: boolean;
};

type Query = { installation: Installation | null; page: number };
type Result = { query: Query } & (
  | { installations: Page<Installation> }
  | { repositories: Page<GithubRepositoryItem> }
  | { problem: unknown }
);

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
  const [selected, setSelected] = useState<{
    item: GithubRepositoryItem;
    scope: RepositoryScope;
  }>();
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
      ? invoke<Page<GithubRepositoryItem>>("github_repositories", {
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

  if (selected) {
    return (
      <GithubRepository
        item={selected.item}
        scope={selected.scope}
        onBack={() => {
          setSelected(undefined);
          setQuery({ ...query });
        }}
      />
    );
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
          {describeGithubProblem(current.problem)}
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
              <div>
                <strong>{item.fullName}</strong>
                <p className="inventory-meta">
                  {visibility[item.visibility]}
                  {item.archived ? " · Archivado" : ""}
                </p>
              </div>
              {query.installation && (
                <button
                  className="button"
                  type="button"
                  aria-label={`Ver actividad de ${item.fullName}`}
                  onClick={() => {
                    if (!query.installation) return;
                    setSelected({
                      item,
                      scope: {
                        githubId,
                        installationId: query.installation.id,
                        repositoryId: item.id,
                        repositoryPage: query.page,
                      },
                    });
                  }}
                >
                  Ver actividad
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {current && (
        <GithubPageActions
          page={query.page}
          hasNext={page?.hasNext ?? false}
          failed={"problem" in current}
          onPage={(next) => setQuery({ ...query, page: next })}
          onRefresh={() => setQuery({ ...query })}
        />
      )}
    </section>
  );
}
