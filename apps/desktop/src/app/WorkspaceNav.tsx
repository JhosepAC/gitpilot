import { useLanguage } from "./language";

export type Workspace =
  "setup" | "identities" | "directories" | "github" | "activity" | "settings";

export function WorkspaceNav({
  current,
  onNavigate,
  disabled,
}: {
  current: Workspace;
  onNavigate: (workspace: Workspace) => void;
  disabled?: boolean;
}) {
  const { t } = useLanguage();
  const sections: Record<Workspace, string> = {
    setup: t("Inicio", "Home"),
    identities: t("Identidades", "Identities"),
    directories: t("Directorios", "Directories"),
    github: "GitHub",
    activity: t("Actividad", "Activity"),
    settings: t("Configuración", "Settings"),
  };
  return (
    <nav
      className="workspace-nav"
      aria-label={t("Secciones de GitPilot", "GitPilot sections")}
    >
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
