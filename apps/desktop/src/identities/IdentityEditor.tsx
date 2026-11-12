import { useState } from "react";
import type { FormEvent } from "react";
import type { IdentityInput } from "./identityApi";
import { PathPicker } from "../app/PathPicker";

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
  const [input, setInput] = useState<IdentityInput>({
    label: initial.label,
    gitName: initial.gitName,
    email: initial.email,
    signingKey: initial.signingKey ?? "",
    signingFormat: initial.signingFormat ?? "ssh",
    sshKey: initial.sshKey ?? "",
    signingRequired: initial.signingRequired ?? Boolean(initial.signingKey),
  });

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!busy) {
      onSave({
        label: input.label,
        gitName: input.gitName,
        email: input.email,
        signingKey: input.signingKey?.trim() ? input.signingKey.trim() : null,
        signingFormat: input.signingKey?.trim()
          ? (input.signingFormat ?? "ssh")
          : null,
        sshKey: input.sshKey?.trim() ? input.sshKey.trim() : null,
        signingRequired: Boolean(
          input.signingKey?.trim() && input.signingRequired,
        ),
      });
    }
  }

  return (
    <>
      <h2>{title}</h2>
      <p className="panel-intro">
        {imported
          ? "Revisa los datos de tu configuración global de Git antes de guardarlos como perfil."
          : "La etiqueta identifica el perfil en GitPilot. Configura el autor y opcionalmente las claves SSH para push y firma de commits."}
      </p>
      <form className="identity-form" onSubmit={submit}>
        <div className="editor-group">
          <label className="field">
            Etiqueta
            <input
              required
              autoComplete="off"
              spellCheck={false}
              placeholder="p. ej. Trabajo"
              value={input.label}
              onChange={(e) =>
                setInput({ ...input, label: e.currentTarget.value })
              }
              disabled={busy}
            />
          </label>
          <label className="field">
            Nombre de Git
            <input
              required
              autoComplete="off"
              spellCheck={false}
              placeholder="p. ej. Ana López"
              value={input.gitName}
              onChange={(e) =>
                setInput({ ...input, gitName: e.currentTarget.value })
              }
              disabled={busy}
            />
          </label>
          <label className="field">
            Correo de Git
            <input
              required
              autoComplete="off"
              inputMode="email"
              spellCheck={false}
              placeholder="p. ej. ana@empresa.test"
              value={input.email}
              onChange={(e) =>
                setInput({ ...input, email: e.currentTarget.value })
              }
              disabled={busy}
            />
          </label>
        </div>

        <div className="editor-section">
          <div className="section-header">
            <h3>Autenticación remota (SSH Push / Fetch)</h3>
            <span className="section-badge">Recomendado</span>
          </div>
          <p className="field-help">
            GitPilot configurará <code>core.sshCommand</code> en los
            repositorios de este perfil para que al hacer <code>git push</code>{" "}
            se use esta clave SSH y no la global.
          </p>
          <PathPicker
            label="Clave privada de autenticación SSH"
            value={input.sshKey ?? ""}
            onChange={(sshKey) => setInput({ ...input, sshKey })}
            disabled={busy}
            clearable
          />
          <p className="field-help">
            Sin clave, el acceso SSH quedará bloqueado al aplicar este perfil.
            HTTPS conserva su propio mecanismo de credenciales.
          </p>
        </div>

        <div className="editor-section">
          <div className="section-header">
            <h3>Firma de commits</h3>
            <span className="section-badge optional">Opcional</span>
          </div>
          <p className="field-help">
            La clave de firma es independiente de la autenticación remota.
            Puedes configurarla sin exigir firma en cada commit.
          </p>
          <div className="field-row">
            <label className="field field-format">
              Formato
              <select
                value={input.signingFormat ?? "ssh"}
                onChange={(e) =>
                  setInput({
                    ...input,
                    signingFormat: e.currentTarget.value as "ssh" | "openpgp",
                    signingKey: "",
                  })
                }
                disabled={busy}
              >
                <option value="ssh">SSH (ssh-keygen)</option>
                <option value="openpgp">OpenPGP (GPG)</option>
              </select>
            </label>
            {input.signingFormat === "openpgp" ? (
              <label className="field field-key">
                Clave de firma
                <input
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={
                    input.signingFormat === "openpgp"
                      ? "ID de clave o huella GPG"
                      : "p. ej. ~/.ssh/id_ed25519_trabajo.pub"
                  }
                  value={input.signingKey ?? ""}
                  onChange={(e) =>
                    setInput({ ...input, signingKey: e.currentTarget.value })
                  }
                  disabled={busy}
                />
              </label>
            ) : (
              <PathPicker
                label="Clave de firma SSH"
                value={input.signingKey ?? ""}
                onChange={(signingKey) => setInput({ ...input, signingKey })}
                disabled={busy}
                clearable
              />
            )}
          </div>
          <label className="field-help">
            <input
              type="checkbox"
              checked={Boolean(input.signingRequired)}
              disabled={busy || !input.signingKey}
              onChange={(event) =>
                setInput({
                  ...input,
                  signingRequired: event.currentTarget.checked,
                })
              }
            />{" "}
            Exigir firma en los commits de este perfil
          </label>
        </div>

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
