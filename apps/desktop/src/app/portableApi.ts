import { invoke } from "@tauri-apps/api/core";
import type { Language } from "./language";

type Scan = {
  timeSeconds: number;
  maxEntries: number;
  maxDepth: number;
  maxRepositories: number;
};

type Preferences = { language: Language; scan: Scan };

export type ImportReview = {
  token: string;
  profiles: {
    label: string;
    gitName: string;
    email: string;
    reuse: boolean;
    makeDefault: boolean;
    keepLocalDefault: boolean;
  }[];
  rules: {
    scope: "directory" | "repository";
    path: string;
    profile: string;
    decision: "add" | "existing" | "conflict" | "missing";
  }[];
  before: Preferences;
  after: Preferences;
};

export const portableApi = {
  readLanguage: () => invoke<Language>("read_language"),
  saveLanguage: (language: Language) =>
    invoke<void>("save_language", { language }),
  preview: (contents: string) =>
    invoke<ImportReview>("preview_portable_settings", { contents }),
  confirm: (contents: string, token: string) =>
    invoke<Preferences>("import_portable_settings", { contents, token }),
  export: () =>
    invoke<"cancelled" | "saved" | "saved_without_activity">(
      "export_portable_settings",
    ),
};
