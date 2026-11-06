import { invoke } from "@tauri-apps/api/core";
import { identityApi } from "../identities/identityApi";

export type Directory = {
  id: number;
  path: string;
  revision: number;
  scanPartial: boolean | null;
  identityId: number | null;
};

export type Repository = {
  id: number;
  path: string;
  revision: number;
  kind: "ordinary" | "worktree" | "separate" | "bare";
  gitDir: string;
  commonDir: string;
  present: boolean;
  identityId: number | null;
};

type ConfigValue = { value: string; scope: string; origin: string };

export type ObservedFields = {
  name: ConfigValue | null;
  email: ConfigValue | null;
};

export type NativePreview = {
  gitVersion: string;
  globalPath: string | null;
  globalAppend: string | null;
  manifestPath: string;
  rules: {
    path: string;
    source: "directory" | "default" | "repository";
    pattern: string;
    profilePath: string;
    profile: { label: string; gitName: string; email: string };
  }[];
  comparisons: {
    path: string;
    expected: string | null;
    current: ObservedFields;
    proposed: ObservedFields;
    result:
      "matches" | "conflict" | "unexpected" | "diagnostic_only" | "unchanged";
  }[];
  findings: { code: string; path: string | null }[];
};
export type Diagnosis = {
  expected: {
    label: string;
    gitName: string;
    email: string;
    source: "directory" | "repository" | "default";
    directory: string | null;
  } | null;
  observed:
    | (ObservedFields & {
        author: string | null;
        committer: string | null;
      })
    | null;
  observationError: string | null;
  conflict: boolean;
};

type Contents = { exclusions: string[]; repositories: Repository[] };
type ScanReport = { found: number; skipped: number; partial: boolean };

export const inventoryApi = {
  preview: () => invoke<NativePreview>("preview_native_config"),
  add: (path: string) => invoke<number>("add_directory", { path }),
  remove: ({ id, revision }: Directory) =>
    invoke<void>("remove_directory", { id, revision }),
  exclude: ({ id, revision }: Directory, path: string, excluded: boolean) =>
    invoke<void>("set_directory_exclusion", { id, revision, path, excluded }),
  scan: (id: number) => invoke<ScanReport>("scan_directory", { id }),
  assign: (
    { id, revision }: Directory | Repository,
    repository: boolean,
    identityId: number | null,
  ) =>
    invoke<void>("set_inventory_identity", {
      id,
      revision,
      repository,
      identityId,
    }),
};

export async function loadInventory(
  directoryId?: number,
  repositoryId?: number,
) {
  const [directories, profiles] = await Promise.all([
    invoke<Directory[]>("list_directories"),
    identityApi.list(),
  ]);
  const directory =
    directories.find(({ id }) => id === directoryId) ?? directories[0];
  const contents = directory
    ? await invoke<Contents>("list_directory_contents", { id: directory.id })
    : { exclusions: [], repositories: [] };
  const repository = contents.repositories.find(
    ({ id }) => id === repositoryId,
  );
  const diagnosis = repository
    ? await invoke<Diagnosis>("diagnose_repository", { id: repository.id })
    : null;
  return { directories, profiles, directory, contents, repository, diagnosis };
}

export function inventoryError(error: unknown) {
  switch (error) {
    case "invalid_path":
      return "Comprueba que la ruta sea absoluta, exista y sea accesible.";
    case "outside_directory":
      return "La exclusión debe ser una subcarpeta del directorio seleccionado.";
    case "already_registered":
      return "Ese directorio ya está registrado.";
    case "not_found":
    case "conflict":
      return "Los datos cambiaron. Actualiza la vista antes de intentarlo de nuevo.";
    case "scan_in_progress":
      return "Ya hay un escaneo en curso.";
    case "git_unavailable":
      return "GitPilot no encontró Git en PATH.";
    case "git_read_failed":
      return "Git no pudo verificar el repositorio. Comprueba su configuración y vuelve a escanear.";
    default:
      return "No se pudo completar la operación en el almacenamiento local.";
  }
}

export function folderName(path: string) {
  return (
    path
      .replace(/[\\/]+$/, "")
      .split(/[\\/]/)
      .at(-1) || path
  );
}
