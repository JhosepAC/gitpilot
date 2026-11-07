import { useEffect, useRef, useState } from "react";
import { Overview } from "./Overview";
import type { Workspace } from "./WorkspaceNav";
import { useLanguage } from "./language";
import type { Language } from "./language";
import { portableApi } from "./portableApi";
import type { ImportReview } from "./portableApi";

const languageNames = { es: "Español", en: "English" };
const maxBytes = 1024 * 1024;

export function SettingsPage({
  onNavigate,
  onLanguageChange,
}: {
  onNavigate: (workspace: Workspace) => void;
  onLanguageChange: (language: Language) => void;
}) {
  const { language, t } = useLanguage();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  const [notice, setNotice] = useState<
    "export" | "exportUnlogged" | "import" | "language"
  >();
  const [pending, setPending] = useState<{
    contents: string;
    review: ImportReview;
  }>();
  const input = useRef<HTMLInputElement>(null);
  const openFile = useRef<HTMLButtonElement>(null);
  const reviewTitle = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    if (pending) reviewTitle.current?.focus();
  }, [pending]);

  const errors: Record<string, string> = {
    file_too_large: t(
      "El archivo supera el límite de 1 MiB.",
      "The file exceeds the 1 MiB limit.",
    ),
    invalid_file: t(
      "El archivo contiene datos inválidos. Elige un JSON exportado por GitPilot.",
      "The file contains invalid data. Choose a JSON file exported by GitPilot.",
    ),
    unsupported_version: t(
      "Esta versión de GitPilot no admite el formato del archivo.",
      "This version of GitPilot does not support the file format.",
    ),
    stale_preview: t(
      "Los datos locales cambiaron. Actualiza la revisión antes de importar.",
      "Local data changed. Refresh the review before importing.",
    ),
    file_exists: t(
      "Ya existe un archivo con ese nombre. Elige un nombre nuevo.",
      "A file with that name already exists. Choose a new name.",
    ),
    file_write_failed: t(
      "No se pudo guardar el archivo. Elige otra ubicación.",
      "The file could not be saved. Choose another location.",
    ),
    dialog_failed: t(
      "No se pudo abrir el selector de archivos.",
      "The file picker could not be opened.",
    ),
  };
  const notices = {
    export: t("Archivo exportado.", "File exported."),
    exportUnlogged: t(
      "Archivo exportado. No se pudo registrar en Actividad.",
      "File exported. The activity entry could not be saved.",
    ),
    import: t("Configuración importada.", "Settings imported."),
    language: t("Idioma guardado.", "Language saved."),
  };

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setNotice(undefined);
    setError(undefined);
    try {
      await action();
    } catch (problem) {
      setError(problem);
    } finally {
      setBusy(false);
    }
  }

  function closeReview() {
    setPending(undefined);
    requestAnimationFrame(() => openFile.current?.focus());
  }

  return (
    <Overview
      workspace="settings"
      title={t("Configuración", "Settings")}
      subtitle={t(
        "Idioma y datos locales de GitPilot.",
        "GitPilot language and local data.",
      )}
      onNavigate={onNavigate}
      busy={busy}
    >
      <div className="identity-feedback">
        <div role="status">
          {busy ? t("Procesando…", "Working…") : notice && notices[notice]}
        </div>
        {error !== undefined && (
          <p role="alert">
            {errors[String(error)] ??
              t(
                "No se pudo completar la operación. Vuelve a intentarlo.",
                "The operation could not be completed. Try again.",
              )}
          </p>
        )}
      </div>
      {pending ? (
        <section aria-labelledby="import-review">
          <h2 id="import-review" tabIndex={-1} ref={reviewTitle}>
            {t("Revisar importación", "Review import")}
          </h2>
          <ImportDetails review={pending.review} />
          <div className="settings-actions import-actions">
            <button
              type="button"
              className="button"
              disabled={busy}
              onClick={() => {
                setError(undefined);
                closeReview();
              }}
            >
              {t("Cancelar", "Cancel")}
            </button>
            {error === "stale_preview" ? (
              <button
                type="button"
                className="button primary"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    setPending({
                      ...pending,
                      review: await portableApi.preview(pending.contents),
                    });
                  })
                }
              >
                {t("Actualizar revisión", "Refresh review")}
              </button>
            ) : (
              <button
                type="button"
                className="button primary"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    const result = await portableApi.confirm(
                      pending.contents,
                      pending.review.token,
                    );
                    onLanguageChange(result.language);
                    closeReview();
                    setNotice("import");
                  })
                }
              >
                {t("Confirmar importación", "Confirm import")}
              </button>
            )}
          </div>
        </section>
      ) : (
        <div className="settings-sections">
          <section aria-labelledby="language">
            <h2 id="language">{t("Idioma", "Language")}</h2>
            <div className="settings-row">
              <label htmlFor="interface-language">
                {t("Idioma de la interfaz", "Interface language")}
              </label>
              <select
                id="interface-language"
                value={language}
                disabled={busy}
                onChange={(event) => {
                  const value = event.currentTarget.value as Language;
                  void run(async () => {
                    await portableApi.saveLanguage(value);
                    onLanguageChange(value);
                    setNotice("language");
                  });
                }}
              >
                <option value="es" lang="es">
                  Español
                </option>
                <option value="en" lang="en">
                  English
                </option>
              </select>
            </div>
            <p className="panel-intro settings-language-note">
              {t(
                "Traduce Inicio, Configuración, Actividad y la navegación.",
                "Applies to Home, Settings, Activity, and navigation.",
              )}
            </p>
          </section>
          <section aria-labelledby="portable-settings">
            <h2 id="portable-settings">{t("Portabilidad", "Portability")}</h2>
            <p className="panel-intro">
              {t(
                "Transfiere perfiles de identidad, asignaciones, idioma y límites de escaneo. El archivo incluye correos y rutas locales.",
                "Transfer identity profiles, assignments, language, and scan limits. The file contains email addresses and local paths.",
              )}
            </p>
            <div className="settings-actions">
              <button
                type="button"
                className="button"
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    const outcome = await portableApi.export();
                    if (outcome !== "cancelled")
                      setNotice(
                        outcome === "saved" ? "export" : "exportUnlogged",
                      );
                  })
                }
              >
                {t("Exportar JSON", "Export JSON")}
              </button>
              <button
                type="button"
                className="button primary"
                ref={openFile}
                disabled={busy}
                onClick={() => input.current?.click()}
              >
                {t("Elegir archivo para importar", "Choose file to import")}
              </button>
              <input
                type="file"
                ref={input}
                accept=".json,application/json"
                hidden
                onChange={(event) => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = "";
                  if (!file) return;
                  void run(async () => {
                    if (file.size > maxBytes) throw "file_too_large";
                    const bytes = await file.arrayBuffer();
                    let contents: string;
                    try {
                      contents = new TextDecoder("utf-8", {
                        fatal: true,
                      }).decode(bytes);
                    } catch {
                      throw "invalid_file";
                    }
                    setPending({
                      contents,
                      review: await portableApi.preview(contents),
                    });
                  });
                }}
              />
            </div>
          </section>
        </div>
      )}
    </Overview>
  );
}

function ImportDetails({ review }: { review: ImportReview }) {
  const { t } = useLanguage();
  const decisions = {
    add: t("Añadir asignación", "Add assignment"),
    existing: t("Ya asignado", "Already assigned"),
    conflict: t("Conservar asignación local", "Keep local assignment"),
    missing: t("Ruta ausente del inventario", "Path missing from inventory"),
  };
  const existing = review.profiles.filter((profile) => profile.reuse).length;
  const added = review.rules.filter((rule) => rule.decision === "add").length;
  const preferences = [
    [
      t("Idioma", "Language"),
      languageNames[review.before.language],
      languageNames[review.after.language],
    ],
    [
      t("Tiempo de escaneo (s)", "Scan time (s)"),
      review.before.scan.timeSeconds,
      review.after.scan.timeSeconds,
    ],
    [
      t("Entradas máximas", "Max entries"),
      review.before.scan.maxEntries,
      review.after.scan.maxEntries,
    ],
    [
      t("Profundidad máxima", "Max depth"),
      review.before.scan.maxDepth,
      review.after.scan.maxDepth,
    ],
    [
      t("Repositorios máximos", "Max repositories"),
      review.before.scan.maxRepositories,
      review.after.scan.maxRepositories,
    ],
  ].filter(([, before, after]) => before !== after);
  return (
    <>
      <p className="panel-intro">
        {t(
          "La confirmación guarda los datos en GitPilot. Para aplicarlos a Git, abre la vista previa en Directorios.",
          "Confirmation saves the data in GitPilot. To apply it to Git, open the preview in Directories.",
        )}
      </p>
      <div className="settings-sections import-sections">
        <details open>
          <summary>
            {t(
              `Perfiles · Nuevos ${review.profiles.length - existing}, reutilizados ${existing}`,
              `Profiles · ${review.profiles.length - existing} new, ${existing} reused`,
            )}
          </summary>
          {review.profiles.length > 0 && (
            <ul className="overview-list">
              {review.profiles.map((profile, index) => (
                <li key={index}>
                  <div>
                    <strong>{profile.label}</strong>
                    <p>
                      {profile.gitName} · {profile.email}
                    </p>
                    <p className="inventory-meta">
                      {profile.reuse
                        ? t("Reutilizar", "Reuse")
                        : t("Añadir", "Add")}
                      {profile.makeDefault &&
                        t(" · Marcar como predeterminado", " · Set as default")}
                      {profile.keepLocalDefault &&
                        t(
                          " · Conservar el perfil predeterminado local",
                          " · Keep the local default profile",
                        )}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </details>
        <details open>
          <summary>
            {t(
              `Asignaciones · ${added} de ${review.rules.length} se añadirán`,
              `Assignments · ${added} of ${review.rules.length} will be added`,
            )}
          </summary>
          {review.rules.length > 0 && (
            <ul className="overview-list">
              {review.rules.map((rule) => (
                <li key={`${rule.scope}:${rule.path}`}>
                  <div>
                    <strong>{rule.profile}</strong>
                    <p>{rule.path}</p>
                    <p className="inventory-meta">
                      {rule.scope === "directory"
                        ? t("Directorio", "Directory")
                        : t("Repositorio", "Repository")}{" "}
                      · {decisions[rule.decision]}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </details>
        <details open>
          <summary>
            {t("Cambios en preferencias", "Preference changes")}
          </summary>
          {preferences.length ? (
            <dl>
              {preferences.map(([label, before, after]) => (
                <div className="settings-row" key={label}>
                  <dt>{label}</dt>
                  <dd>
                    {before} → {after}
                  </dd>
                </div>
              ))}
            </dl>
          ) : (
            <p className="panel-intro">
              {t(
                "Se conservan las preferencias actuales.",
                "Current preferences are kept.",
              )}
            </p>
          )}
        </details>
      </div>
    </>
  );
}
