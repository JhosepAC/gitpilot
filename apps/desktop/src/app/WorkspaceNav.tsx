type Workspace = "identities" | "directories";

export function WorkspaceNav({
  current,
  onNavigate,
  disabled,
}: {
  current: Workspace;
  onNavigate: (workspace: Workspace) => void;
  disabled?: boolean;
}) {
  return (
    <nav className="workspace-nav" aria-label="Secciones de GitPilot">
      <button
        type="button"
        aria-current={current === "identities" ? "page" : undefined}
        disabled={disabled}
        onClick={() => onNavigate("identities")}
      >
        Identidades
      </button>
      <button
        type="button"
        aria-current={current === "directories" ? "page" : undefined}
        disabled={disabled}
        onClick={() => onNavigate("directories")}
      >
        Directorios
      </button>
    </nav>
  );
}
