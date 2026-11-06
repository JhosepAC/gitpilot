export type Workspace = "setup" | "identities" | "directories" | "activity";

const sections: Record<Workspace, string> = {
  setup: "Inicio",
  identities: "Identidades",
  directories: "Directorios",
  activity: "Actividad",
};

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
      {(Object.keys(sections) as Workspace[]).map((section) => (
        <button
          key={section}
          type="button"
          aria-current={current === section ? "page" : undefined}
          disabled={disabled}
          onClick={() => onNavigate(section)}
        >
          {sections[section]}
        </button>
      ))}
    </nav>
  );
}
