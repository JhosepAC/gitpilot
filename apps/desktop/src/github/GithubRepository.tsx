import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { GithubRemoteLinks } from "./GithubRemoteLinks";
import {
  describeGithubProblem,
  GithubPageActions,
  visibility,
  type GithubRepositoryItem,
  type Page,
  type RepositoryScope,
} from "./githubResourceUi";

const views = {
  pulls: "Pull requests",
  issues: "Issues",
  links: "Vínculo local",
};
type View = keyof typeof views;
type Kind = Exclude<View, "links">;
type ActivityItem = {
  number: number;
  title: string;
  state: "open" | "closed" | "draft";
};
const states = { open: "Abierto", closed: "Cerrado", draft: "Borrador" };

export function GithubRepository({
  item,
  scope,
  onBack,
}: {
  item: GithubRepositoryItem;
  scope: RepositoryScope;
  onBack: () => void;
}) {
  const [view, setView] = useState<View>("pulls");
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);

  return (
    <section
      className="github-resources"
      aria-labelledby="github-repository-title"
    >
      <div className="github-resource-heading">
        <div>
          <h2 id="github-repository-title" ref={heading} tabIndex={-1}>
            {item.fullName}
          </h2>
          <p className="inventory-meta">
            {visibility[item.visibility]}
            {item.archived ? " · Archivado" : ""}
          </p>
        </div>
        <button className="button" type="button" onClick={onBack}>
          Volver a repositorios
        </button>
      </div>
      <div
        className="github-repository-views"
        role="group"
        aria-label="Contenido del repositorio"
      >
        {(Object.keys(views) as View[]).map((choice) => (
          <button
            className="button"
            type="button"
            key={choice}
            aria-pressed={view === choice}
            onClick={() => setView(choice)}
          >
            {views[choice]}
          </button>
        ))}
      </div>
      {view === "links" ? (
        <GithubRemoteLinks scope={scope} fullName={item.fullName} />
      ) : (
        <GithubActivity key={view} kind={view} scope={scope} />
      )}
    </section>
  );
}

function GithubActivity({
  kind,
  scope,
}: {
  kind: Kind;
  scope: RepositoryScope;
}) {
  const [query, setQuery] = useState({ page: 1 });
  const [result, setResult] = useState<
    { query: typeof query } & (
      { data: Page<ActivityItem> } | { problem: unknown }
    )
  >();
  const current = result?.query === query ? result : undefined;
  const data = current && "data" in current ? current.data : undefined;
  const heading = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    let active = true;
    void invoke<Page<ActivityItem>>("github_repository_activity", {
      scope,
      kind,
      page: query.page,
    }).then(
      (data) => {
        if (active) setResult({ query, data });
      },
      (problem: unknown) => {
        if (active) setResult({ query, problem });
      },
    );
    return () => {
      active = false;
    };
  }, [kind, scope, query]);

  function changePage(page: number) {
    setQuery({ page });
    heading.current?.focus();
  }

  return (
    <section
      className="github-repository-content"
      aria-labelledby="github-repository-content"
    >
      <h3 id="github-repository-content" ref={heading} tabIndex={-1}>
        {views[kind]}
      </h3>
      {!current && (
        <p className="inventory-meta" role="status">
          Consultando GitHub…
        </p>
      )}
      {current && "problem" in current && (
        <p className="inventory-conflict" role="alert">
          {describeGithubProblem(current.problem)}
        </p>
      )}
      {data?.partialSso && (
        <p className="inventory-meta" role="status">
          GitHub devolvió resultados parciales por SSO.
        </p>
      )}
      {data?.items.length === 0 && (
        <p className="inventory-meta" role="status">
          {data.hasNext || query.page > 1
            ? `Esta página no contiene ${views[kind].toLowerCase()}.`
            : `No hay ${views[kind].toLowerCase()} visibles en este repositorio.`}
        </p>
      )}
      {data && data.items.length > 0 && (
        <ul className="github-resource-list">
          {data.items.map((item) => (
            <li className="github-resource-row" key={item.number}>
              <div>
                <strong>{item.title || "Sin título"}</strong>
                <p className="inventory-meta">
                  #{item.number} · {states[item.state]}
                </p>
              </div>
            </li>
          ))}
        </ul>
      )}
      {current && (
        <GithubPageActions
          page={query.page}
          hasNext={data?.hasNext ?? false}
          failed={"problem" in current}
          onPage={changePage}
          onRefresh={() => changePage(query.page)}
        />
      )}
    </section>
  );
}
