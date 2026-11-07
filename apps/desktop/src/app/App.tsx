import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { FoundationState } from "./foundation";
import { readFoundationStatus } from "./foundation";
import { IdentitiesPage } from "../identities/IdentitiesPage";
import { Brand } from "./Brand";
import { InventoryPage } from "../inventory/InventoryPage";
import { ActivityPage, loadSetup, SetupPage } from "./Overview";
import type { Workspace } from "./WorkspaceNav";
import { GithubPage } from "../github/GithubPage";
import { SettingsPage } from "./SettingsPage";
import { LanguageContext } from "./language";
import type { Language } from "./language";
import { portableApi } from "./portableApi";

const messages = {
  loading: "Comprobando el almacenamiento local…",
  preview:
    "Abre GitPilot en una ventana de escritorio para comprobar el almacenamiento.",
};

export function App() {
  const [state, setState] = useState<FoundationState>({ kind: "loading" });
  const [workspace, setWorkspace] = useState<Workspace>("identities");
  const [language, setLanguage] = useState<Language>("es");
  const focusWorkspace = useRef(false);

  useEffect(() => {
    let active = true;
    void readFoundationStatus().then(async (nextState) => {
      if (nextState.kind === "ready") {
        const [setup, savedLanguage] = await Promise.all([
          loadSetup().catch(() => undefined),
          portableApi.readLanguage().catch(() => "es" as const),
        ]);
        if (active) {
          setLanguage(savedLanguage);
          if (setup && !setup.identities && !setup.directories)
            setWorkspace("setup");
        }
      }
      if (active) setState(nextState);
    });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  useLayoutEffect(() => {
    if (focusWorkspace.current) {
      document.getElementById("workspace-title")?.focus();
      focusWorkspace.current = false;
    }
  }, [workspace]);

  function navigate(next: Workspace) {
    if (next === workspace) return;
    focusWorkspace.current = true;
    setWorkspace(next);
  }

  if (state.kind === "ready") {
    return (
      <LanguageContext.Provider value={language}>
        {workspace === "setup" && <SetupPage onNavigate={navigate} />}
        {workspace === "activity" && <ActivityPage onNavigate={navigate} />}
        {workspace === "github" && <GithubPage onNavigate={navigate} />}
        {workspace === "identities" && <IdentitiesPage onNavigate={navigate} />}
        {workspace === "directories" && <InventoryPage onNavigate={navigate} />}
        {workspace === "settings" && (
          <SettingsPage onNavigate={navigate} onLanguageChange={setLanguage} />
        )}
      </LanguageContext.Provider>
    );
  }

  return (
    <main className="startup">
      <Brand />
      <section className="startup-status" aria-labelledby="storage">
        <h1 id="storage">Almacenamiento local</h1>
        <p role={state.kind === "error" ? "alert" : "status"}>
          {state.kind === "error" ? state.message : messages[state.kind]}
        </p>
      </section>
    </main>
  );
}
