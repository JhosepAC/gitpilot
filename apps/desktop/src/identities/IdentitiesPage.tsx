import { useEffect, useRef, useState } from "react";
import { Brand } from "../app/Brand";
import { useLanguage } from "../app/language";
import { WorkspaceNav } from "../app/WorkspaceNav";
import type { Workspace } from "../app/WorkspaceNav";
import { IdentityEditor } from "./IdentityEditor";
import { identityApi, identityError } from "./identityApi";
import type { Identity, IdentityInput } from "./identityApi";
import { IdentityAssignments } from "./IdentityAssignments";
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

export function IdentitiesPage({
  onNavigate,
}: {
  onNavigate: (workspace: Workspace) => void;
}) {
  const { t } = useLanguage();
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
  const empty = profiles?.length === 0 && pane.kind === "detail" && !loadError;

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
        let savedId: number;
        if (!profile) {
          savedId = await identityApi.create(input);
        } else {
          await identityApi.update(profile, input);
          savedId = profile.id;
        }
        return savedId;
      },
      profile
        ? "Cambios guardados. Reaplica el perfil desde Directorios para actualizar Git."
        : "Identidad creada.",
    );
  }

  const actions = (
    <div className="action-row">
      <button className="button" disabled={locked} onClick={importFromGit}>
        Importar desde Git
      </button>
      <button
        className="button primary"
        disabled={locked}
        onClick={() => show({ kind: "edit", input: emptyInput })}
      >
        {t("Nueva identidad", "New identity")}
      </button>
    </div>
  );

  return (
    <main className="identity-shell">
      <header className="identity-heading">
        <div>
          <h1 id="workspace-title" tabIndex={-1}>
            {t("Identidades", "Identities")}
          </h1>
          <p>{t("Perfiles de autor de Git.", "Git author profiles.")}</p>
        </div>
        {!empty && pane.kind === "detail" && actions}
      </header>
      <aside
        className="identity-sidebar"
        aria-label={t("Perfiles guardados", "Saved profiles")}
      >
        <Brand />
        <WorkspaceNav
          current="identities"
          onNavigate={onNavigate}
          disabled={busy || pane.kind !== "detail"}
        />
        <div className="list-heading">
          <h2>{t("Perfiles", "Profiles")}</h2>
          {profiles && <span>{profiles.length}</span>}
        </div>
        {profiles && profiles.length > 0 ? (
          <ul className="profile-list">
            {profiles.map((profile) => (
              <li key={profile.id}>
                <button
                  className="profile-choice"
                  aria-current={active?.id === profile.id ? "true" : undefined}
                  disabled={locked}
                  onClick={() => show({ kind: "detail", id: profile.id })}
                >
                  <span className="profile-label">{profile.label}</span>
                  <span className="profile-meta">{profile.email}</span>
                  {profile.isDefault && (
                    <span className="profile-default">
                      {t("Predeterminada", "Default")}
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="profile-empty">
            {profiles
              ? t("Todavía no hay perfiles.", "No profiles yet.")
              : loadError
                ? t("Lista no disponible.", "List unavailable.")
                : t("Cargando perfiles…", "Loading profiles…")}
          </p>
        )}
        <footer className="sidebar-foot">
          {t("Almacenamiento local", "Local storage")}
        </footer>
      </aside>
      <section
        className="identity-panel"
        ref={detail}
        tabIndex={-1}
        aria-label="Detalle de la identidad"
        lang="es"
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
              Se eliminará el perfil local <strong>{pane.profile.label}</strong>
              .
            </p>
            <div className="action-row">
              <button
                className="button"
                disabled={busy}
                onClick={() => show({ kind: "detail", id: pane.profile.id })}
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
            <div className="commit-author">
              <h3>Autor del commit</h3>
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
            </div>
            <div className="commit-author ssh-auth-card">
              <h3>Autenticación remota (SSH)</h3>
              <dl className="identity-values">
                <div>
                  <dt>Clave SSH para push / fetch</dt>
                  <dd className={active.sshKey ? "code-value" : undefined}>
                    {active.sshKey ||
                      "Sin clave · SSH bloqueado al aplicar el perfil"}
                  </dd>
                </div>
              </dl>
            </div>
            <div className="commit-author signing-card">
              <h3>Firma de commits</h3>
              <dl className="identity-values">
                <div>
                  <dt>Estado de firma</dt>
                  <dd>
                    {active.signingKey
                      ? `${active.signingRequired ? "Obligatoria" : "Opcional"} · ${active.signingFormat === "openpgp" ? "OpenPGP (GPG)" : "SSH (ssh-keygen)"}`
                      : "Desactivada"}
                  </dd>
                </div>
                {active.signingKey && (
                  <div>
                    <dt>Clave pública / ID</dt>
                    <dd className="code-value">{active.signingKey}</dd>
                  </div>
                )}
              </dl>
            </div>
            <div className="identity-usage">
              <div>
                <h3>Uso en GitPilot</h3>
                {active.isDefault && <p>Perfil predeterminado.</p>}
                {!active.isDefault && active.assignmentCount === 0 && (
                  <p>Sin asignaciones activas.</p>
                )}
                {active.assignmentCount > 0 && (
                  <p>
                    {active.assignmentCount === 1
                      ? "1 asignación local."
                      : `${active.assignmentCount} asignaciones locales.`}
                  </p>
                )}
              </div>
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  change(
                    async () => {
                      await identityApi.setDefault(active, !active.isDefault);
                      return active.id;
                    },
                    active.isDefault
                      ? "Preferencia predeterminada retirada."
                      : "Predeterminado guardado. Revisa y aplica la configuración desde Directorios.",
                  )
                }
              >
                {active.isDefault
                  ? "Quitar como predeterminada"
                  : "Usar como predeterminada"}
              </button>
            </div>
            <IdentityAssignments
              key={active.id}
              identity={active}
              onNavigate={onNavigate}
            />
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
                disabled={
                  busy || active.isDefault || active.assignmentCount > 0
                }
                onClick={() => show({ kind: "delete", profile: active })}
              >
                Eliminar
              </button>
              {active.assignmentCount > 0 ? (
                <small>
                  Cambia las asignaciones en Directorios antes de eliminar este
                  perfil.
                </small>
              ) : (
                active.isDefault && (
                  <small>
                    Quita la preferencia para poder eliminar este perfil.
                  </small>
                )
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
          <div className="empty-identities">
            <h2>Crea tu primera identidad</h2>
            <p className="panel-intro">
              Separa tus commits personales de los de trabajo. Crea un perfil o
              trae el nombre y el correo que ya usas en Git.
            </p>
            {actions}
          </div>
        ) : (
          <p role="status">Cargando identidades…</p>
        )}
      </section>
    </main>
  );
}
