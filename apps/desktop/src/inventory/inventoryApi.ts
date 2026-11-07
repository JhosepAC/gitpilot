import { Channel, invoke } from "@tauri-apps/api/core";
import { identityApi } from "../identities/identityApi";
import type { InspectorData } from "./RepositoryInspector";

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
  confirmation: string | null;
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
export type NativeOperation = {
  id: number;
  local: boolean;
  hook: boolean;
  state: "applied" | "recovery_required";
  target: string;
  confirmation: string | null;
};
export type LocalPreview = {
  target: string;
  confirmation: string;
  append: string;
  name: string;
  email: string;
};
export type ActivityItem = {
  id: number;
  target: string;
  state:
    | "preparing"
    | "applied"
    | "restoring"
    | "restored"
    | "rolled_back"
    | "kept"
    | "exported"
    | "imported";
  scope: "local" | "global" | "hook" | "data";
  at: string;
  profileCount: number | null;
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
  conflictFingerprint: string | null;
  kept: boolean;
};

type Contents = { exclusions: string[]; repositories: Repository[] };
export type ScanReport = {
  found: number;
  skipped: number;
  visited: number;
  partial: boolean;
  cancelled: boolean;
  limits: ("time" | "entries" | "depth" | "repositories")[];
};

export type ScanProgress = Pick<ScanReport, "found" | "skipped" | "visited"> & {
  phase: "walking" | "saving";
};

export type ScanPreferences = {
  timeSeconds: number;
  maxEntries: number;
  maxDepth: number;
  maxRepositories: number;
};

export type ScanSettings = {
  values: ScanPreferences;
  defaults: ScanPreferences;
  minimum: ScanPreferences;
  maximum: ScanPreferences;
};

export const inventoryApi = {
  directories: () => invoke<Directory[]>("list_directories"),
  contents: (id: number) => invoke<Contents>("list_directory_contents", { id }),
  inspect: (id: number) => invoke<InspectorData>("inspect_repository", { id }),
  preview: () => invoke<NativePreview>("preview_native_config"),
  nativeStatus: (operationId?: number) =>
    invoke<NativeOperation | null>("native_config_status", { operationId }),
  confirmNative: (
    confirmation: string,
    restore: boolean,
    context?: { repositoryId?: number; operationId?: number },
  ) =>
    invoke<void>("confirm_native_config", {
      confirmation,
      restore,
      ...context,
    }),
  activity: () => invoke<ActivityItem[]>("native_activity"),
  previewLocal: (id: number) =>
    invoke<LocalPreview>("preview_local_identity", { id }),
  keep: (id: number, fingerprint: string) =>
    invoke<void>("keep_repository_identity", { id, fingerprint }),
  add: (path: string) => invoke<number>("add_directory", { path }),
  remove: ({ id, revision }: Directory) =>
    invoke<void>("remove_directory", { id, revision }),
  exclude: ({ id, revision }: Directory, path: string, excluded: boolean) =>
    invoke<void>("set_directory_exclusion", { id, revision, path, excluded }),
  scan: (
    id: number,
    scanId: string,
    onProgress: (progress: ScanProgress) => void,
  ) =>
    invoke<ScanReport>("scan_directory", {
      id,
      scanId,
      onProgress: new Channel(onProgress),
    }),
  cancelScan: (scanId: string) =>
    invoke<boolean>("cancel_directory_scan", { scanId }),
  scanPreferences: () => invoke<ScanSettings>("read_scan_preferences"),
  saveScanPreferences: (expected: ScanPreferences, values: ScanPreferences) =>
    invoke<ScanSettings>("save_scan_preferences", { expected, values }),
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
    inventoryApi.directories(),
    identityApi.list(),
  ]);
  const directory =
    directories.find(({ id }) => id === directoryId) ?? directories[0];
  const contents = directory
    ? await inventoryApi.contents(directory.id)
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
    case "scan_preferences_invalid":
      return "Revisa que los límites sean números enteros dentro de los intervalos indicados.";
    case "scan_preferences_changed":
      return "Las preferencias cambiaron en otra ventana. Recárgalas antes de guardar.";
    case "scan_request_invalid":
      return "No se pudo iniciar el escaneo. Vuelve a intentarlo.";
    case "stale_preview":
      return "La vista previa venció. Vuelve al inventario y ábrela de nuevo.";
    case "busy":
      return "Git u otra instancia mantiene un bloqueo. Espera y vuelve a intentarlo.";
    case "active_operation":
      return "Ya hay una aplicación activa para este destino. Puedes revisarla y restaurarla desde Actividad.";
    case "identity_required":
      return "Asigna un perfil al repositorio antes de preparar el hook.";
    case "unsafe_target":
      return "GitPilot no pudo comprobar el destino de forma segura.";
    case "backup_unavailable":
      return "El respaldo cifrado no está disponible. Comprueba el almacén de secretos del sistema.";
    case "verification_failed":
      return "Git no confirmó el resultado. GitPilot restauró el estado previo.";
    case "recovery_required":
      return "GitPilot detuvo la operación y conservó el respaldo. Revisa el estado antes de continuar.";
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
