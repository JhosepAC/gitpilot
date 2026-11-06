import { useEffect, useState } from "react";
import type { FoundationState } from "./foundation";
import { readFoundationStatus } from "./foundation";
import { IdentitiesPage } from "../identities/IdentitiesPage";
import { Brand } from "./Brand";

const messages = {
  loading: "Comprobando el almacenamiento local…",
  preview:
    "Abre GitPilot en una ventana de escritorio para comprobar el almacenamiento.",
};

export function App() {
  const [state, setState] = useState<FoundationState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    void readFoundationStatus().then((nextState) => {
      if (active) setState(nextState);
    });
    return () => {
      active = false;
    };
  }, []);

  if (state.kind === "ready") return <IdentitiesPage />;

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
