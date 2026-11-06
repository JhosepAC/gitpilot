import { useEffect, useState } from "react";
import type { FoundationState } from "./foundation";
import { readFoundationStatus } from "./foundation";
import { IdentitiesPage } from "../identities/IdentitiesPage";

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
    <main className="mx-auto max-w-2xl px-6 py-14 sm:px-12">
      <h1 className="text-3xl font-semibold tracking-tight">GitPilot</h1>
      <p className="mt-2 text-muted">Gestión de identidades Git</p>
      <section
        className="mt-10 border-t border-divider pt-5"
        aria-labelledby="storage"
      >
        <h2 id="storage" className="font-medium">
          Almacenamiento local
        </h2>
        <p className="mt-2 max-w-prose text-muted" role="status">
          {state.kind === "error" ? state.message : messages[state.kind]}
        </p>
      </section>
    </main>
  );
}
