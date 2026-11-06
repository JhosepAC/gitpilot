import { useEffect, useRef, useState } from "react";
import { IdentityEditor } from "./IdentityEditor";
import { identityApi, identityError } from "./identityApi";
import type { Identity, IdentityInput } from "./identityApi";
import "./identities.css";

type Pane =
  | { kind: "detail"; id?: number }
  | {
      kind: "edit";
      profile?: Identity;
      input: IdentityInput;
      imported?: boolean;
    }
  | { kind: "delete"; profile: Identity };

const emptyInput = { label: "", gitName: "", email: "" };

function BranchIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      width="20"
      height="20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="6" cy="5" r="2.5" />
      <circle cx="6" cy="19" r="2.5" />
      <circle cx="18" cy="5" r="2.5" />
      <path d="M6 7.5v9M18 7.5v1A5.5 5.5 0 0 1 12.5 14H6" />
    </svg>
  );
}

export function IdentitiesPage() {
  const [profiles, setProfiles] = useState<Identity[] | null>(null);
  const [pane, setPane] = useState<Pane>({ kind: "detail" });
  const [busy, setBusy] = useState(false);
  const [loadError, setLoadError] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const detail = useRef<HTMLElement>(null);
  const focusDetail = useRef(false);
  const active =
    pane.kind === "detail"
      ? (profiles?.find((profile) => profile.id === pane.id) ?? profiles?.[0])
      : pane.profile;
  const locked =
    busy || loadError || profiles === null || pane.kind !== "detail";

  useEffect(() => {
    let mounted = true;
    void identityApi.list().then(
      (items) => {
        if (mounted) setProfiles(items);
      },
      () => {
        if (mounted) setLoadError(true);
      },
    );
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (focusDetail.current) {
      detail.current?.focus();
      focusDetail.current = false;
    }
  }, [pane]);

  function show(next: Pane) {
    focusDetail.current = true;
    setPane(next);
    setError("");
    setNotice("");
  }

  async function refresh() {
    try {
      setProfiles(await identityApi.list());
      setLoadError(false);
    } catch {
      setLoadError(true);
    }
  }

  async function perform(operation: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await operation();
    } catch (problem) {
      setError(identityError(problem));
    } finally {
      setBusy(false);
    }
  }

  function change(write: () => Promise<number | undefined>, message: string) {
    void perform(async () => {
      const id = await write();
      show({ kind: "detail", id });
      setNotice(message);
      await refresh();
    });
  }

  function importFromGit() {
    void perform(async () => {
      const input = await identityApi.detect();
      show({ kind: "edit", input, imported: true });
    });
  }

  function save(input: IdentityInput) {
    if (pane.kind !== "edit") return;
    const profile = pane.profile;
    change(
      async () => {
        if (!profile) return identityApi.create(input);
        await identityApi.update(profile, input);
        return profile.id;
      },
      profile ? "Cambios guardados." : "Identidad creada.",
    );
  }

  return (
    <div className="identity-shell">
      <header className="identity-bar">
        <div className="identity-brand">
          <BranchIcon /> GitPilot
        </div>
        <span>Perfiles locales</span>
      </header>
      <main className="identity-main">
        <div className="identity-heading">
          <div>
            <h1>Identidades</h1>
            <p>Organiza los perfiles que usarás en tus commits.</p>
          </div>
          <div className="action-row">
            <button
              className="button"
              disabled={locked}
              onClick={importFromGit}
            >
              Importar desde Git
            </button>
            <button
              className="button primary"
              disabled={locked}
              onClick={() => show({ kind: "edit", input: emptyInput })}
            >
              Nueva identidad
            </button>
          </div>
        </div>
        <div className="identity-workspace">
          <aside className="identity-sidebar" aria-label="Perfiles guardados">
            <div className="list-heading">
              <h2>Perfiles guardados</h2>
              {profiles && <span>{profiles.length}</span>}
            </div>
            {profiles && profiles.length > 0 ? (
              <ul className="profile-list">
                {profiles.map((profile) => (
                  <li key={profile.id}>
                    <button
                      className="profile-choice"
                      aria-current={
                        active?.id === profile.id ? "true" : undefined
                      }
                      disabled={locked}
                      onClick={() => show({ kind: "detail", id: profile.id })}
                    >
                      <span className="profile-label">{profile.label}</span>
                      <span className="profile-meta">
                        {profile.isDefault ? "Predeterminada" : profile.email}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="profile-empty">
                {profiles
                  ? "Todavía no hay perfiles."
                  : loadError
                    ? "Lista no disponible."
                    : "Cargando perfiles…"}
              </p>
            )}
          </aside>
          <section
            className="identity-panel"
            ref={detail}
            tabIndex={-1}
            aria-label="Detalle de la identidad"
          >
            <div className="identity-feedback">
              <div role="status">{busy ? "Procesando…" : notice}</div>
              {error && <p role="alert">{error}</p>}
            </div>
            {loadError ? (
              <>
                <h2>No se pudieron cargar los perfiles</h2>
                <p className="panel-intro">
                  Actualiza la lista para volver a consultar el almacenamiento
                  local.
                </p>
                <button
                  className="button"
                  disabled={busy}
                  onClick={() => void perform(refresh)}
                >
                  Actualizar lista
                </button>
              </>
            ) : pane.kind === "edit" ? (
              <IdentityEditor
                initial={pane.input}
                title={
                  pane.profile
                    ? "Editar identidad"
                    : pane.imported
                      ? "Revisar identidad de Git"
                      : "Nueva identidad"
                }
                imported={pane.imported ?? false}
                busy={busy}
                onCancel={() => {
                  show({ kind: "detail", id: pane.profile?.id });
                  if (error) void perform(refresh);
                }}
                onSave={save}
              />
            ) : pane.kind === "delete" ? (
              <>
                <h2>Eliminar identidad</h2>
                <p className="panel-intro">
                  Se eliminará el perfil local{" "}
                  <strong>{pane.profile.label}</strong>.
                </p>
                <div className="action-row">
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      show({ kind: "detail", id: pane.profile.id })
                    }
                  >
                    Cancelar
                  </button>
                  <button
                    className="button danger"
                    disabled={busy}
                    onClick={() =>
                      change(async () => {
                        await identityApi.remove(pane.profile);
                        return undefined;
                      }, "Identidad eliminada.")
                    }
                  >
                    Eliminar perfil
                  </button>
                </div>
              </>
            ) : active ? (
              <>
                <div className="detail-heading">
                  <h2>{active.label}</h2>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      show({ kind: "edit", profile: active, input: active })
                    }
                  >
                    Editar
                  </button>
                </div>
                <dl className="identity-values">
                  <div>
                    <dt>Nombre de Git</dt>
                    <dd>{active.gitName}</dd>
                  </div>
                  <div>
                    <dt>Correo de Git</dt>
                    <dd>{active.email}</dd>
                  </div>
                </dl>
                <div className="identity-usage">
                  <h3>Uso en GitPilot</h3>
                  <p>
                    {active.isDefault
                      ? "Perfil predeterminado."
                      : "Sin asignaciones activas."}
                  </p>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      change(
                        async () => {
                          await identityApi.setDefault(
                            active,
                            !active.isDefault,
                          );
                          return active.id;
                        },
                        active.isDefault
                          ? "Preferencia predeterminada retirada."
                          : "Identidad predeterminada actualizada.",
                      )
                    }
                  >
                    {active.isDefault
                      ? "Quitar como predeterminada"
                      : "Usar como predeterminada"}
                  </button>
                </div>
                <div className="action-row detail-actions">
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      change(
                        () => identityApi.duplicate(active),
                        "Identidad duplicada.",
                      )
                    }
                  >
                    Duplicar
                  </button>
                  <button
                    className="button danger"
                    disabled={busy || active.isDefault}
                    onClick={() => show({ kind: "delete", profile: active })}
                  >
                    Eliminar
                  </button>
                  {active.isDefault && (
                    <small>
                      Quita la preferencia para poder eliminar este perfil.
                    </small>
                  )}
                </div>
                {error && (
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() => void perform(refresh)}
                  >
                    Actualizar lista
                  </button>
                )}
              </>
            ) : profiles ? (
              <>
                <div className="empty-mark">
                  <BranchIcon />
                </div>
                <h2>Crea tu primera identidad</h2>
                <p className="panel-intro">
                  Añade un perfil para separar, por ejemplo, tus proyectos
                  personales de los de trabajo. También puedes copiar el nombre
                  y el correo que ya tienes en Git.
                </p>
              </>
            ) : (
              <p role="status">Cargando identidades…</p>
            )}
          </section>
        </div>
      </main>
    </div>
  );
}
