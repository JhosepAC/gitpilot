import { useState } from "react";
import { useLanguage } from "../app/language";
import type { Workspace } from "../app/WorkspaceNav";
import type { Repository } from "../inventory/inventoryApi";
import { inventoryApi } from "../inventory/inventoryApi";
import type { Identity } from "./identityApi";

type Assignment = {
  key: string;
  path: string;
  repository: boolean;
  present: boolean;
};
type State =
  | { kind: "idle" | "loading" | "error" }
  | { kind: "ready"; items: Assignment[] };

export function IdentityAssignments({
  identity,
  onNavigate,
}: {
  identity: Identity;
  onNavigate: (workspace: Workspace) => void;
}) {
  const { t } = useLanguage();
  const [state, setState] = useState<State>({ kind: "idle" });

  async function load() {
    setState({ kind: "loading" });
    try {
      const directories = await inventoryApi.directories();
      const contents = await Promise.all(
        directories.map((directory) => inventoryApi.contents(directory.id)),
      );
      const items: Assignment[] = directories
        .filter((directory) => directory.identityId === identity.id)
        .map((directory) => ({
          key: `directory-${directory.id}`,
          path: directory.path,
          repository: false,
          present: true,
        }));
      const repositories = new Map<number, Repository>();
      for (const repository of contents.flatMap(
        (content) => content.repositories,
      )) {
        const previous = repositories.get(repository.id);
        repositories.set(repository.id, {
          ...repository,
          present: repository.present || Boolean(previous?.present),
        });
      }
      for (const repository of repositories.values()) {
        if (repository.identityId === identity.id)
          items.push({
            key: `repository-${repository.id}`,
            path: repository.path,
            repository: true,
            present: repository.present,
          });
      }
      items.sort(
        (a, b) =>
          Number(a.repository) - Number(b.repository) ||
          a.path.localeCompare(b.path),
      );
      setState({ kind: "ready", items });
    } catch {
      setState({ kind: "error" });
    }
  }

  return (
    <details
      className="identity-assignments"
      onToggle={(event) => {
        if (event.currentTarget.open && state.kind === "idle") void load();
      }}
    >
      <summary>
        {t("Ver asignaciones", "View assignments")}{" "}
        <span>({identity.assignmentCount})</span>
      </summary>
      <div className="assignment-content">
        <p>
          {t(
            "Reglas explícitas de esta identidad. Los repositorios dentro de un directorio heredan su perfil salvo que tengan una excepción.",
            "Explicit rules for this identity. Repositories inside a directory inherit its profile unless they have an override.",
          )}
        </p>
        {identity.isDefault && (
          <p>
            {t(
              "También es el perfil predeterminado para repositorios del inventario sin una regla más específica.",
              "Also the default profile for inventory repositories without a more specific rule.",
            )}
          </p>
        )}
        {state.kind === "loading" && (
          <p role="status">
            {t("Cargando asignaciones…", "Loading assignments…")}
          </p>
        )}
        {state.kind === "error" && (
          <div role="alert">
            <p>
              {t(
                "No se pudieron consultar las asignaciones.",
                "Could not load assignments.",
              )}
            </p>
            <button className="button" onClick={() => void load()}>
              {t("Reintentar", "Retry")}
            </button>
          </div>
        )}
        {state.kind === "ready" &&
          (state.items.length ? (
            <ul className="assignment-list">
              {state.items.map((item) => (
                <li key={item.key}>
                  <span className="assignment-kind">
                    {item.repository
                      ? t("Excepción de repositorio", "Repository override")
                      : t("Directorio", "Directory")}
                    {!item.present && t(" · No encontrado", " · Not found")}
                  </span>
                  <span className="assignment-path">{item.path}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p>
              {t(
                "Esta identidad no tiene asignaciones explícitas.",
                "This identity has no explicit assignments.",
              )}
            </p>
          ))}
        <button className="button" onClick={() => onNavigate("directories")}>
          {t("Gestionar en Directorios", "Manage in Directories")}
        </button>
      </div>
    </details>
  );
}
