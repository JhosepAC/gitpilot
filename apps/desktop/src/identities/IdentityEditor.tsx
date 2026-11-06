import { useState } from "react";
import type { FormEvent } from "react";
import type { IdentityInput } from "./identityApi";

const fields = [
  ["label", "Etiqueta", "p. ej. Trabajo"],
  ["gitName", "Nombre de Git", "p. ej. Ana López"],
  ["email", "Correo de Git", "p. ej. ana@empresa.test"],
] as const;

export function IdentityEditor({
  initial,
  title,
  imported,
  busy,
  onCancel,
  onSave,
}: {
  initial: IdentityInput;
  title: string;
  imported: boolean;
  busy: boolean;
  onCancel: () => void;
  onSave: (input: IdentityInput) => void;
}) {
  const [input, setInput] = useState({
    label: initial.label,
    gitName: initial.gitName,
    email: initial.email,
  });

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!busy) onSave(input);
  }

  return (
    <>
      <h2>{title}</h2>
      <p className="panel-intro">
        {imported
          ? "Revisa los datos de tu configuración global de Git antes de guardarlos como perfil."
          : "La etiqueta identifica el perfil en GitPilot. El nombre y el correo identifican al autor en Git."}
      </p>
      <form className="identity-form" onSubmit={submit}>
        {fields.map(([key, label, placeholder]) => (
          <label key={key} className="field">
            {label}
            <input
              required
              autoComplete="off"
              inputMode={key === "email" ? "email" : undefined}
              spellCheck={false}
              placeholder={placeholder}
              value={input[key]}
              onChange={(event) =>
                setInput({ ...input, [key]: event.currentTarget.value })
              }
              disabled={busy}
            />
          </label>
        ))}
        <div className="action-row">
          <button type="submit" className="button primary" disabled={busy}>
            Guardar identidad
          </button>
          <button
            type="button"
            className="button"
            onClick={onCancel}
            disabled={busy}
          >
            Cancelar
          </button>
        </div>
      </form>
    </>
  );
}
