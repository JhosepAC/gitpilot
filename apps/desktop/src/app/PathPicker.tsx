import { useId, useState } from "react";
import type { Ref } from "react";
import { invoke } from "@tauri-apps/api/core";

export function PathPicker({
  label,
  value,
  onChange,
  directory = false,
  disabled = false,
  inputRef,
  clearable = false,
}: {
  label: string;
  value: string;
  onChange: (path: string) => void;
  directory?: boolean;
  disabled?: boolean;
  inputRef?: Ref<HTMLInputElement>;
  clearable?: boolean;
}) {
  const id = useId();
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState("");
  async function select() {
    setOpening(true);
    setError("");
    try {
      const path = await invoke<string | null>("plugin:dialog|open", {
        options: { title: label, directory, multiple: false },
      });
      if (path) onChange(path);
    } catch {
      setError("No se pudo abrir el explorador. Vuelve a seleccionar la ruta.");
    } finally {
      setOpening(false);
    }
  }
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <div className="path-picker">
        <input
          id={id}
          ref={inputRef}
          readOnly
          value={value}
          title={value}
          placeholder="Sin selección"
          disabled={disabled || opening}
        />
        <button
          type="button"
          className="button"
          disabled={disabled || opening}
          onClick={() => void select()}
          aria-label={`Seleccionar ${label.toLocaleLowerCase()}`}
        >
          {opening
            ? "Abriendo…"
            : directory
              ? "Elegir carpeta…"
              : "Elegir archivo…"}
        </button>
        {clearable && value && (
          <button
            type="button"
            className="button"
            disabled={disabled || opening}
            onClick={() => onChange("")}
            aria-label={`Quitar ${label.toLocaleLowerCase()}`}
          >
            Quitar
          </button>
        )}
      </div>
      {error && (
        <p className="field-help" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
