import { useLanguage } from "./language";

export type Workspace =
  "setup" | "identities" | "directories" | "github" | "activity" | "settings";

const icons: Record<Workspace, string> = {
  setup: "M3 10 12 3l9 7M5 9v12h5v-7h4v7h5V9",
  identities:
    "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3a4 4 0 0 1 0 8M22 21v-2a4 4 0 0 0-3-3.87M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
  directories: "M3 7V4h6l2 3h10v13H3V7Z",
  github:
    "M9 19c-4 1-4-2-6-2m12 5v-4a3 3 0 0 0-1-2.3c3-.3 6-1.5 6-6.2a5 5 0 0 0-1.4-3.5 4.6 4.6 0 0 0-.1-3.5S17.4 2.2 15 3.8a13 13 0 0 0-6 0C6.6 2.2 5.5 2.5 5.5 2.5a4.6 4.6 0 0 0-.1 3.5A5 5 0 0 0 4 9.5c0 4.7 3 5.9 6 6.2A3 3 0 0 0 9 18v4",
  activity: "M3 12h4l3-8 4 16 3-8h4",
  settings: "M4 7h16M4 17h16M9 4v6M15 14v6",
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
          <svg
            className="workspace-nav-icon"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
          >
            <path d={icons[section]} />
          </svg>
          <span>{sections[section]}</span>
        </button>
      ))}
    </nav>
  );
}
