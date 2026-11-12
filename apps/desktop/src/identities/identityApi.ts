import { invoke } from "@tauri-apps/api/core";

export type IdentityInput = {
  label: string;
  gitName: string;
  email: string;
  signingKey?: string | null;
  signingFormat?: "ssh" | "openpgp" | null;
  sshKey?: string | null;
  signingRequired?: boolean;
};

export type Identity = IdentityInput & {
  id: number;
  revision: number;
  isDefault: boolean;
  assignmentCount: number;
};

function reference({ id, revision }: Identity) {
  return { id, revision };
}

export const identityApi = {
  list: () => invoke<Identity[]>("list_identities"),
  create: (input: IdentityInput) =>
    invoke<number>("create_identity", { input }),
  update: (profile: Identity, input: IdentityInput) =>
    invoke<void>("update_identity", { ...reference(profile), input }),
  duplicate: (profile: Identity) =>
    invoke<number>("duplicate_identity", reference(profile)),
  setDefault: (profile: Identity, enabled: boolean) =>
    invoke<void>("set_identity_default", { ...reference(profile), enabled }),
  remove: (profile: Identity) =>
    invoke<void>("delete_identity", reference(profile)),
  detect: () => invoke<IdentityInput>("detect_git_identity"),
};

export function identityError(error: unknown): string {
  switch (error) {
    case "invalid_label":
      return "Introduce una etiqueta de hasta 80 caracteres en una sola línea.";
    case "invalid_name":
      return "Introduce un nombre de Git de hasta 120 caracteres sin signos < ni >.";
    case "invalid_email":
      return "Introduce un correo de Git válido, sin espacios y de hasta 254 caracteres.";
    case "invalid_signing_key":
      return "Introduce una clave de firma válida de hasta 500 caracteres sin caracteres de control.";
    case "invalid_ssh_key":
      return "Introduce una ruta de clave SSH válida de hasta 500 caracteres sin caracteres de control.";
    case "conflict":
      return "Esta identidad cambió. Cancela la edición y actualiza la lista.";
    case "in_use":
      return "Esta identidad está en uso. Retira su selección como predeterminada o cambia sus asignaciones antes de eliminarla.";
    case "git_unavailable":
      return "GitPilot no encontró Git. Comprueba que Git esté instalado y disponible en PATH.";
    case "git_identity_missing":
      return "Git no tiene un nombre y un correo globales completos. Puedes crear una identidad manualmente.";
    case "git_read_failed":
      return "No se pudo leer la configuración global de Git. Compruébala o crea una identidad manualmente.";
    default:
      return "No se pudo acceder al almacenamiento local. Inténtalo de nuevo.";
  }
}
