import type { Diagnosis, ObservedFields } from "./inventoryApi";
import { inventoryError } from "./inventoryApi";

const scopes: Record<string, string> = {
  system: "Sistema",
  global: "Global",
  local: "Local",
  worktree: "Worktree",
  command: "Comando",
};

export function GitConfigFields({ identity }: { identity: ObservedFields }) {
  return (
    <dl className="observed-identity">
      {(
        [
          ["Nombre de Git", identity.name],
          ["Correo de Git", identity.email],
        ] as const
      ).map(([label, field]) => (
        <div key={label}>
          <dt className="inventory-meta">{label}</dt>
          <dd>{field?.value ?? "Sin configurar"}</dd>
          {field && (
            <dd className="inventory-meta">
              {scopes[field.scope] ?? field.scope} · {field.origin}
            </dd>
          )}
        </div>
      ))}
    </dl>
  );
}

export function RepositoryDiagnosis({ diagnosis }: { diagnosis: Diagnosis }) {
  const { expected, observed, conflict, observationError } = diagnosis;
  return (
    <>
      {conflict && (
        <p className="inventory-conflict" role="status">
          La identidad esperada difiere de la configuración de Git.
        </p>
      )}
      <div className="commit-author inventory-diagnosis">
        <div>
          <h3>Identidad esperada</h3>
          {expected ? (
            <>
              <strong>{expected.label}</strong>
              <div>{expected.gitName}</div>
              <div>{expected.email}</div>
              <p className="inventory-meta">
                {expected.source === "repository"
                  ? "Excepción del repositorio"
                  : expected.source === "default"
                    ? "Perfil predeterminado"
                    : "Regla del directorio"}
              </p>
              {expected.directory && (
                <div className="inventory-meta">{expected.directory}</div>
              )}
            </>
          ) : (
            <p className="inventory-meta">Sin identidad asignada.</p>
          )}
        </div>
        <div>
          <h3>Configuración de Git</h3>
          {observed ? (
            <GitConfigFields identity={observed} />
          ) : (
            <p className="inventory-meta">
              {observationError
                ? inventoryError(observationError)
                : "Repositorio ausente del último escaneo."}
            </p>
          )}
        </div>
      </div>
      {observed && (observed.author || observed.committer) && (
        <details className="inventory-disclosure">
          <summary>Autor y committer observados</summary>
          <dl className="observed-identity">
            <div>
              <dt className="inventory-meta">Autor</dt>
              <dd>{observed.author ?? "Sin determinar"}</dd>
            </div>
            <div>
              <dt className="inventory-meta">Committer</dt>
              <dd>{observed.committer ?? "Sin determinar"}</dd>
            </div>
          </dl>
          <p className="inventory-meta">
            Estos valores corresponden al entorno en el que se ejecuta GitPilot.
          </p>
        </details>
      )}
    </>
  );
}
