import { useEffect, useState } from "react";
import type { FoundationState } from "./foundation";
import { readFoundationStatus } from "./foundation";
import { IdentitiesPage } from "../identities/IdentitiesPage";
import { Brand } from "./Brand";
import { InventoryPage } from "../inventory/InventoryPage";
import { ActivityPage, loadSetup, SetupPage } from "./Overview";
import type { Workspace } from "./WorkspaceNav";

const messages = {
  loading: "Comprobando el almacenamiento local…",
  preview:
    "Abre GitPilot en una ventana de escritorio para comprobar el almacenamiento.",
};

export function App() {
  const [state, setState] = useState<FoundationState>({ kind: "loading" });
  const [workspace, setWorkspace] = useState<Workspace>("identities");

  useEffect(() => {
    let active = true;
    void readFoundationStatus().then(async (nextState) => {
      if (nextState.kind === "ready") {
        const setup = await loadSetup().catch(() => undefined);
        if (active && setup && !setup.identities && !setup.directories)
          setWorkspace("setup");
      }
      if (active) setState(nextState);
    });
    return () => {
      active = false;
    };
  }, []);

  if (state.kind === "ready") {
    if (workspace === "setup") return <SetupPage onNavigate={setWorkspace} />;
    if (workspace === "activity")
      return <ActivityPage onNavigate={setWorkspace} />;
    return workspace === "identities" ? (
      <IdentitiesPage onNavigate={setWorkspace} />
    ) : (
      <InventoryPage onNavigate={setWorkspace} />
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
