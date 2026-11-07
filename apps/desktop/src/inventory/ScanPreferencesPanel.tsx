import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  inventoryApi,
  inventoryError,
  type ScanPreferences,
  type ScanSettings,
} from "./inventoryApi";

const fields = [
  { key: "timeSeconds", label: "Tiempo máximo (segundos)" },
  { key: "maxEntries", label: "Entradas máximas" },
  { key: "maxDepth", label: "Profundidad máxima" },
  { key: "maxRepositories", label: "Repositorios máximos" },
] as const;

type Field = (typeof fields)[number]["key"];
type Draft = Record<Field, string>;

function edit(values: ScanPreferences): Draft {
  return {
    timeSeconds: String(values.timeSeconds),
    maxEntries: String(values.maxEntries),
    maxDepth: String(values.maxDepth),
    maxRepositories: String(values.maxRepositories),
  };
}

export function ScanPreferencesPanel({ onBack }: { onBack: () => void }) {
  const [settings, setSettings] = useState<ScanSettings>();
  const [draft, setDraft] = useState<Draft>();
  const [request, setRequest] = useState(0);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState("");
  const [failure, setFailure] = useState<{
    message: string;
    reload?: boolean;
  }>();
  const heading = useRef<HTMLHeadingElement>(null);
  const form = useRef<HTMLFormElement>(null);
  const changed =
    settings &&
    draft &&
    fields.some(({ key }) => draft[key] !== String(settings.values[key]));

  useEffect(() => {
    heading.current?.focus();
  }, []);

  useEffect(() => {
    let active = true;
    void inventoryApi.scanPreferences().then(
      (next) => {
        if (!active) return;
        setSettings(next);
        setDraft(edit(next.values));
      },
      () => {
        if (active)
          setFailure({
            message: "No se pudieron leer las preferencias de escaneo.",
            reload: true,
          });
      },
    );
    return () => {
      active = false;
    };
  }, [request]);

  function reload() {
    setFailure(undefined);
    setNotice("");
    setSettings(undefined);
    setDraft(undefined);
    setRequest((previous) => previous + 1);
    heading.current?.focus();
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!settings || !draft || !changed || saving) return;
    setNotice("");
    const invalid = fields.find(({ key }) => {
      const value = Number(draft[key]);
      return (
        draft[key] === "" ||
        !Number.isSafeInteger(value) ||
        value < settings.minimum[key] ||
        value > settings.maximum[key]
      );
    });
    if (invalid) {
      setFailure({ message: inventoryError("scan_preferences_invalid") });
      form.current
        ?.querySelector<HTMLInputElement>(`[name="${invalid.key}"]`)
        ?.focus();
      return;
    }
    setFailure(undefined);
    setSaving(true);
    try {
      const saved = await inventoryApi.saveScanPreferences(settings.values, {
        timeSeconds: Number(draft.timeSeconds),
        maxEntries: Number(draft.maxEntries),
        maxDepth: Number(draft.maxDepth),
        maxRepositories: Number(draft.maxRepositories),
      });
      setSettings(saved);
      setDraft(edit(saved.values));
      setNotice("Preferencias guardadas.");
    } catch (problem) {
      setFailure({
        message: inventoryError(problem),
        reload: problem === "scan_preferences_changed",
      });
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <button
        type="button"
        className="button inventory-back"
        disabled={saving}
        onClick={onBack}
      >
        Volver al inventario
      </button>
      <h2 className="preview-title" tabIndex={-1} ref={heading}>
        Preferencias de escaneo
      </h2>
      <p className="panel-intro">
        Estos límites se aplican a los próximos escaneos de todos los
        directorios.
      </p>
      {!settings && !failure && (
        <p className="inventory-meta" role="status">
          Cargando preferencias…
        </p>
      )}
      {settings && draft && (
        <form ref={form} onSubmit={(event) => void save(event)} noValidate>
          <div className="inventory-scan-fields">
            {fields.map(({ key, label }) => (
              <label className="field" key={key}>
                <span>{label}</span>
                <input
                  name={key}
                  type="number"
                  step={1}
                  min={settings.minimum[key]}
                  max={settings.maximum[key]}
                  value={draft[key]}
                  disabled={saving}
                  aria-describedby={`scan-range-${key}`}
                  onChange={(event) => {
                    setDraft({ ...draft, [key]: event.currentTarget.value });
                    setNotice("");
                    if (!failure?.reload) setFailure(undefined);
                  }}
                />
                <span className="inventory-meta" id={`scan-range-${key}`}>
                  De {settings.minimum[key].toLocaleString()} a{" "}
                  {settings.maximum[key].toLocaleString()}.
                  {key === "maxDepth"
                    ? " Con 0 se comprueba solo la raíz."
                    : ""}
                </span>
              </label>
            ))}
          </div>
          <div className="action-row">
            <button
              className="button primary"
              type="submit"
              disabled={!changed || saving}
            >
              {saving ? "Guardando…" : "Guardar preferencias"}
            </button>
            <button
              className="button"
              type="button"
              disabled={saving}
              onClick={() => {
                setDraft(edit(settings.defaults));
                if (!failure?.reload) setFailure(undefined);
                setNotice("");
              }}
            >
              Usar valores predeterminados
            </button>
          </div>
        </form>
      )}
      {failure && (
        <>
          <p className="inventory-conflict" role="alert">
            {failure.message}
          </p>
          {failure.reload && (
            <div className="action-row">
              <button className="button" type="button" onClick={reload}>
                Recargar preferencias
              </button>
            </div>
          )}
        </>
      )}
      {notice && (
        <p className="inventory-scan-notice" role="status">
          {notice}
        </p>
      )}
    </>
  );
}
