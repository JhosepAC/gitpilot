import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { readFoundationStatus } from "./foundation";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));

describe("readFoundationStatus", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(isTauri).mockReturnValue(true);
  });

  it("reads the database status through the permitted command", async () => {
    vi.mocked(invoke).mockResolvedValue({ database: "ready" });
    await expect(readFoundationStatus()).resolves.toEqual({ kind: "ready" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("foundation_status");
  });

  it("keeps browser preview independent from native IPC", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(readFoundationStatus()).resolves.toEqual({ kind: "preview" });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("turns storage startup failures into an actionable message", async () => {
    vi.mocked(invoke).mockRejectedValue("storage_initialization_failed");
    await expect(readFoundationStatus()).resolves.toMatchObject({
      kind: "error",
      message: expect.stringContaining("reinicia GitPilot"),
    });
  });

  it("keeps raw IPC errors out of the view", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("private diagnostic detail"));
    await expect(readFoundationStatus()).resolves.toEqual({
      kind: "error",
      message:
        "No se pudo verificar el almacenamiento local. Reinicia GitPilot.",
    });
  });
});
