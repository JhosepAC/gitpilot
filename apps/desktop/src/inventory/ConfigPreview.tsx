import { useEffect, useRef, useState } from "react";
import type { NativeOperation, NativePreview } from "./inventoryApi";
import { GitConfigFields } from "./RepositoryDiagnosis";

const priorities = {
  directory: "Regla del directorio",
  default: "Perfil predeterminado",
  repository: "Excepción del repositorio",
};

const findings: Record<string, string> = {
  unsupported_git: "La simulación requiere Git 2.39.5 o posterior.",
  redirected_config:
    "El entorno redirige la configuración o el repositorio de Git. Abre GitPilot en un entorno Git normal para simular.",
  unknown_global: "No se pudo verificar la ruta de la configuración global.",
  unverified_directory:
    "Escanea esta carpeta por completo para ampliar la verificación.",
  unsupported_path:
    "La ruta queda fuera del alcance que GitPilot puede simular.",
  unsupported_exception:
    "La excepción requiere un repositorio ordinario presente.",
  comparison_limit:
    "La simulación alcanzó su límite. Quedan repositorios por verificar.",
  unverified_repository:
    "Git no pudo comprobar este repositorio. Revisa su configuración y vuelve a escanear.",
  unverified_worktree: "Git no pudo comprobar todos los worktrees vinculados.",
  managed_config_present:
    "Ya hay una referencia administrada en el archivo global.",
  unsafe_global:
    "El archivo global o su carpeta requieren revisión antes de aplicar.",
};

const results = {
  matches: "Coincide con la identidad esperada",
  conflict: "La discrepancia seguiría presente",
  unexpected: "Efecto fuera de la regla esperada",
  diagnostic_only: "Solo diagnóstico",
  unchanged: "Sin cambios previstos",
};

export function ConfigPreview({
  preview,
  onBack,
  onConfirm,
  busy,
}: {
  preview: NativePreview;
  onBack: () => void;
  onConfirm: (confirmation: string) => void;
  busy: boolean;
}) {
  const [review, setReview] = useState(false);
  const title = useRef<HTMLHeadingElement>(null);
  const reviewTitle = useRef<HTMLHeadingElement>(null);
  useEffect(() => title.current?.focus({ preventScroll: true }), []);
  useEffect(() => reviewTitle.current?.focus(), [review]);
  const issues = preview.comparisons.filter(
    ({ result }) => result === "conflict" || result === "unexpected",
  );
  const ordered = [
    ...issues,
    ...preview.comparisons.filter((item) => !issues.includes(item)),
  ];
  return (
    <>
      <button
        className="button inventory-back"
        onClick={onBack}
        disabled={busy}
      >
        Volver al inventario
      </button>
      <h2 className="preview-title" tabIndex={-1} ref={title}>
        Vista previa de Git
      </h2>
      <p className="panel-intro">
        Simulación de las reglas de todos los directorios. Tus archivos Git se
        conservan tal como están.
      </p>
      {issues.length > 0 && (
        <p className="inventory-conflict">
          Repositorios con discrepancias o efectos inesperados · {issues.length}
        </p>
      )}
      {preview.rules.length > 0 && (
        <div className="commit-author preview-destinations">
          <h3>Archivos propuestos</h3>
          <dl className="observed-identity">
            <div>
              <dt className="inventory-meta">
                Configuración global · añadir una referencia
              </dt>
              <dd>{preview.globalPath ?? "Destino sin verificar"}</dd>
            </div>
            <div>
              <dt className="inventory-meta">
                Manifiesto · activar las reglas
              </dt>
              <dd>{preview.manifestPath}</dd>
            </div>
          </dl>
          {preview.globalAppend && (
            <details className="inventory-disclosure">
              <summary>Referencia que se añadiría al archivo global</summary>
              <pre className="preview-diff">{preview.globalAppend}</pre>
            </details>
          )}
        </div>
      )}
      {preview.confirmation &&
        (review ? (
          <div className="inventory-section">
            <h3 ref={reviewTitle} tabIndex={-1}>
              Confirmar aplicación
            </h3>
            <p>
              GitPilot respaldará y verificará este archivo antes de activar las
              reglas.
            </p>
            <p className="inventory-meta">{preview.globalPath}</p>
            <div className="action-row preview-actions">
              <button
                className="button primary"
                disabled={busy}
                onClick={() =>
                  preview.confirmation && onConfirm(preview.confirmation)
                }
              >
                Confirmar y aplicar
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() => setReview(false)}
              >
                Cancelar
              </button>
            </div>
          </div>
        ) : (
          <button
            className="button primary preview-actions"
            onClick={() => setReview(true)}
            disabled={busy}
          >
            Revisar aplicación
          </button>
        ))}
      <p className="inventory-meta preview-version">{preview.gitVersion}</p>
      {preview.findings.length > 0 && (
        <div className="inventory-section">
          <h3>Revisar antes de aplicar</h3>
          <ul className="preview-findings">
            {preview.findings.map(({ code, path }) => (
              <li key={code + path}>
                {findings[code] ?? "La simulación requiere revisión."}
                {path && <div className="inventory-meta">{path}</div>}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="inventory-section">
        <h3>
          Resultado por repositorio <span>{ordered.length}</span>
        </h3>
        {ordered.length === 0 && (
          <p className="inventory-meta">Sin comparaciones disponibles.</p>
        )}
        {ordered.map((item) => (
          <details className="inventory-disclosure" key={item.path}>
            <summary>
              {item.path}
              <span className="inventory-meta preview-summary">
                {results[item.result]}
                {item.expected && <> · {item.expected}</>}
              </span>
            </summary>
            <div className="commit-author inventory-diagnosis">
              <div>
                <h3>Actual</h3>
                <GitConfigFields identity={item.current} />
              </div>
              <div>
                <h3>Simulada</h3>
                <GitConfigFields identity={item.proposed} />
              </div>
            </div>
          </details>
        ))}
      </div>
      <div className="inventory-section">
        <h3>
          Reglas propuestas <span>{preview.rules.length}</span>
        </h3>
        {preview.rules.length === 0 ? (
          <p className="inventory-meta">
            Asigna una identidad a un directorio o repositorio para preparar una
            regla.
          </p>
        ) : (
          <p className="inventory-meta">
            Las reglas posteriores tienen prioridad. Una regla de carpeta puede
            alcanzar repositorios ordinarios futuros. Las exclusiones del
            escaneo delimitan el inventario y una inclusión de Git puede
            alcanzar esas rutas.
          </p>
        )}
        {preview.rules.map((rule) => (
          <details className="inventory-disclosure" key={rule.pattern}>
            <summary>
              {rule.path} · {rule.profile.label}
            </summary>
            <p className="inventory-meta">{priorities[rule.source]}</p>
            <div>
              {rule.profile.gitName} &lt;{rule.profile.email}&gt;
            </div>
            <code>{rule.pattern}</code>
            <div className="inventory-meta">{rule.profilePath}</div>
          </details>
        ))}
      </div>
    </>
  );
}

export function NativeOperationPanel({
  operation,
  onBack,
  onRestore,
  busy,
  backLabel = "Volver al inventario",
}: {
  operation: NativeOperation;
  onBack: () => void;
  onRestore: (confirmation: string) => void;
  busy: boolean;
  backLabel?: string;
}) {
  const title = useRef<HTMLHeadingElement>(null);
  useEffect(() => title.current?.focus({ preventScroll: true }), []);
  const pending = operation.state === "recovery_required";
  return (
    <>
      <button
        className="button inventory-back"
        onClick={onBack}
        disabled={busy}
      >
        {backLabel}
      </button>
      <h2 className="preview-title" ref={title} tabIndex={-1}>
        {pending ? "Recuperación pendiente" : "Configuración verificada"}
      </h2>
      <p className="panel-intro">
        {pending
          ? "GitPilot conservó el registro y el respaldo. Revisa el archivo y libera cualquier bloqueo de Git. Puedes reabrir GitPilot para reintentar una operación interrumpida."
          : "La restauración devuelve el archivo a su estado anterior a esta operación."}
      </p>
      <div className="commit-author preview-destinations">
        <h3>
          {operation.hook
            ? "Hook pre-commit"
            : operation.local
              ? "Configuración del repositorio"
              : "Configuración global"}
        </h3>
        <p>{operation.target}</p>
      </div>
      {operation.confirmation && (
        <button
          className="button danger preview-actions"
          disabled={busy}
          onClick={() =>
            operation.confirmation && onRestore(operation.confirmation)
          }
        >
          Confirmar restauración
        </button>
      )}
    </>
  );
}
