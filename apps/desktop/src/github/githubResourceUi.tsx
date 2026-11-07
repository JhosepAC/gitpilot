export type Page<T> = { items: T[]; hasNext: boolean; partialSso: boolean };

export type GithubRepositoryItem = {
  id: number;
  fullName: string;
  visibility: "public" | "private" | "internal";
  archived: boolean;
};

export type RepositoryScope = {
  githubId: number;
  installationId: number;
  repositoryId: number;
  repositoryPage: number;
};

export const visibility = {
  public: "Público",
  private: "Privado",
  internal: "Interno",
};

const problems: Record<string, string> = {
  permission_denied:
    "GitHub denegó el acceso. Revisa los permisos de la instalación.",
  installation_unavailable:
    "La instalación dejó de estar disponible o esta cuenta perdió acceso.",
  repository_unavailable:
    "El repositorio cambió de página o dejó de estar accesible. Vuelve a la lista y selecciónalo de nuevo.",
  local_repository_unavailable:
    "GitPilot no pudo comprobar el repositorio local. Revisa su presencia en Directorios.",
  remote_changed:
    "El destino de lectura cambió o ya no coincide con el repositorio autorizado. Revisa el remoto de nuevo.",
  link_changed: "El vínculo cambió. Revisa su estado y vuelve a intentarlo.",
  sso_required:
    "GitHub requiere que inicies sesión con el SSO de la organización. Completa el acceso en GitHub y reintenta.",
  expired: "La sesión venció. Vuelve a la cuenta para autorizarla de nuevo.",
  not_connected:
    "GitHub ya no acepta esta sesión. Vuelve a la cuenta para autorizarla.",
  renewal_uncertain:
    "No se pudo comprobar la renovación. Vuelve a la cuenta para revisar la sesión.",
  secure_store: "Desbloquea el almacén seguro del sistema y reintenta.",
  storage:
    "No se pudo acceder al almacenamiento local. Revisa el estado y reintenta.",
  network:
    "No se pudo contactar con GitHub. Comprueba la conexión y reintenta.",
  busy: "Hay otra operación de GitHub en curso. Espera y reintenta.",
  rate_limited:
    "GitHub limitó la consulta. Espera al menos un minuto antes de reintentar.",
};

export function describeGithubProblem(problem: unknown) {
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
    "No se pudo completar la operación. Vuelve a intentarlo."
  );
}

export function GithubPageActions({
  page,
  hasNext,
  failed,
  onPage,
  onRefresh,
}: {
  page: number;
  hasNext: boolean;
  failed: boolean;
  onPage: (page: number) => void;
  onRefresh: () => void;
}) {
  return (
    <div className="action-row">
      {(page > 1 || hasNext) && (
        <nav className="github-pagination" aria-label="Páginas de resultados">
          <button
            className="button"
            type="button"
            disabled={page === 1}
            onClick={() => onPage(page - 1)}
          >
            Anterior
          </button>
          <span className="inventory-meta">Página {page}</span>
          <button
            className="button"
            type="button"
            disabled={!hasNext}
            onClick={() => onPage(page + 1)}
          >
            Siguiente
          </button>
        </nav>
      )}
      <button className="button" type="button" onClick={onRefresh}>
        {failed ? "Reintentar" : "Actualizar"}
      </button>
    </div>
  );
}
