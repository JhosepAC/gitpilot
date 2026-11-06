import { invoke, isTauri } from "@tauri-apps/api/core";

type FoundationStatus = { database: "ready" };

export type FoundationState =
  | { kind: "loading" | "ready" | "preview" }
  | { kind: "error"; message: string };

export async function readFoundationStatus(): Promise<FoundationState> {
  if (!isTauri()) return { kind: "preview" };

  try {
    const status = await invoke<FoundationStatus>("foundation_status");
    if (status.database === "ready") return { kind: "ready" };
  } catch (error) {
    if (
      error === "storage_unavailable" ||
      error === "storage_initialization_failed"
    ) {
      return {
        kind: "error",
        message:
          "Comprueba el acceso al almacenamiento local y reinicia GitPilot.",
      };
    }
  }

  return {
    kind: "error",
    message: "No se pudo verificar el almacenamiento local. Reinicia GitPilot.",
  };
}
