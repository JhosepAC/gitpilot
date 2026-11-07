import { useEffect, useRef } from "react";
import { HookControl, ProbeControl } from "./InspectorChecks";

type Endpoint = { transport: string; host: string | null };
type Keys = {
  present: number;
  missing: number;
  restricted: number;
  unchecked: number;
};

export type InspectorData = {
  branch: string | null;
  head: string | null;
  tree: {
    staged: number;
    untracked: number;
    conflicts: number;
    unstaged: number | null;
  } | null;
  remotes: { name: string; fetch: Endpoint[]; push: Endpoint[] }[];
  remotesPartial: boolean;
  ssh: {
    clientAvailable: boolean;
    customCommand: boolean;
    agentSocket: boolean;
    configStatus: "simple" | "absent" | "complex" | "unavailable";
    aliases: { name: string; host: string | null; keys: Keys }[];
    defaultKeys: Keys;
  };
  https: {
    helpers: string[];
    scopedHelpers: boolean;
    askpassConfigured: boolean;
  };
  signing: {
    enabled: boolean | null;
    format: string;
    keyReference: string;
    keyFile: "present" | "missing" | "permissions" | "unchecked" | null;
    customProgram: boolean;
    defaultProgramAvailable: boolean;
  };
};

const transports: Record<string, string> = {
  ssh: "SSH",
  https: "HTTPS",
  http: "HTTP",
  git: "Git",
  local: "Local",
  other: "Otro transporte",
};

const helpers: Record<string, string> = {
  manager: "Git Credential Manager",
  wincred: "Windows Credential Manager",
  libsecret: "Secret Service",
  cache: "Caché en memoria",
  store: "Almacenamiento en disco",
  custom: "Helper personalizado",
};

const sshStatus = {
  simple: "Se revisaron las declaraciones simples del archivo SSH de usuario.",
  absent: "No se encontró un archivo SSH de usuario.",
  complex:
    "La configuración tiene directivas que requieren revisión manual para resolver el alias.",
  unavailable: "No se pudo comprobar el archivo SSH de usuario.",
};

const signingFormats: Record<string, string> = {
  openpgp: "OpenPGP",
  ssh: "SSH",
  x509: "X.509",
  unknown: "Formato desconocido",
};

const keyReference: Record<string, string> = {
  public: "Clave pública SSH configurada",
  file: "Archivo de clave configurado",
  reference: "Referencia de firma configurada",
  command: "Comando selector de clave configurado",
  default: "Selección predeterminada de Git",
};

const keyFile = {
  present: "Archivo encontrado.",
  missing: "Archivo ausente.",
  permissions: "Archivo encontrado con permisos amplios.",
  unchecked: "Disponibilidad del archivo sin comprobar.",
};

function endpoints(values: Endpoint[]) {
  return (
    values
      .map(({ transport, host }) =>
        [transports[transport] ?? transports.other, host]
          .filter(Boolean)
          .join(" · "),
      )
      .join(", ") || "Sin destino comprobado"
  );
}

function KeyCounts({ keys }: { keys: Keys }) {
  return (
    <span>
      {keys.present} {keys.present === 1 ? "presente" : "presentes"}
      {keys.missing > 0 && (
        <>
          {" "}
          · {keys.missing} {keys.missing === 1 ? "ausente" : "ausentes"}
        </>
      )}
      {keys.restricted > 0 && <> · {keys.restricted} con permisos amplios</>}
      {keys.unchecked > 0 && <> · {keys.unchecked} sin verificar</>}
    </span>
  );
}

export function RepositoryInspector({
  data,
  name,
  id,
  ordinary,
  onBack,
}: {
  data: InspectorData;
  name: string;
  id: number;
  ordinary: boolean;
  onBack: () => void;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus({ preventScroll: true }), []);
  const { tree, ssh, https, signing } = data;
  return (
    <>
      <button className="button inventory-back" onClick={onBack}>
        Volver al repositorio
      </button>
      <h2 className="preview-title" tabIndex={-1} ref={heading}>
        Inspector de Git
      </h2>
      <p className="panel-intro">{name} · Estado y comprobaciones de Git.</p>
      <div className="inventory-section">
        <h3>Rama e índice</h3>
        <p>
          {data.branch ?? (data.head ? "HEAD separado" : "Rama sin verificar")}{" "}
          {data.head && <code className="inventory-meta">· {data.head}</code>}
        </p>
        {tree ? (
          <>
            <dl className="inspector-counts">
              <div>
                <dt>Preparados</dt>
                <dd>{tree.staged}</dd>
              </div>
              <div>
                <dt>Sin seguimiento</dt>
                <dd>{tree.untracked}</dd>
              </div>
              <div>
                <dt>Sin preparar</dt>
                <dd>{tree.unstaged ?? "Sin verificar"}</dd>
              </div>
              <div>
                <dt>En conflicto</dt>
                <dd>{tree.conflicts}</dd>
              </div>
            </dl>
            {tree.unstaged === null && (
              <p className="inventory-meta">
                Los filtros externos o un índice incompatible requieren
                comprobar los cambios sin preparar desde Git.
              </p>
            )}
          </>
        ) : (
          <p className="inventory-meta">
            Estado del índice no disponible para este contexto.
          </p>
        )}
      </div>
      <div className="inventory-section">
        <h3>Remotos</h3>
        {data.remotes.length ? (
          <ul className="inspector-remotes">
            {data.remotes.map((remote) => (
              <li key={remote.name}>
                <strong>{remote.name}</strong>
                <dl>
                  <dt>Lectura</dt>
                  <dd>{endpoints(remote.fetch)}</dd>
                  <dt>Envío</dt>
                  <dd>{endpoints(remote.push)}</dd>
                </dl>
                {remote.fetch.length === 1 &&
                  ["github.com", "gitlab.com", "bitbucket.org"].includes(
                    remote.fetch[0].host ?? "",
                  ) &&
                  (remote.fetch[0].transport === "ssh" ||
                    remote.fetch[0].transport === "https") && (
                    <ProbeControl
                      id={id}
                      kind={remote.fetch[0].transport}
                      remote={remote.name}
                    />
                  )}
              </li>
            ))}
          </ul>
        ) : (
          <p className="inventory-meta">Sin remotos visibles.</p>
        )}
        {data.remotesPartial && (
          <p className="inventory-meta" role="status">
            La lista de remotos es parcial.
          </p>
        )}
      </div>
      <div className="inventory-section">
        <h3>Acceso SSH</h3>
        <p>
          {ssh.clientAvailable
            ? "Cliente SSH detectado en PATH."
            : "Cliente SSH sin detectar en PATH."}{" "}
          {ssh.customCommand &&
            "Git usa una configuración de cliente personalizada."}
        </p>
        <p className="inventory-meta">
          {ssh.agentSocket
            ? "El entorno anuncia un socket de agente."
            : "El entorno no anuncia un socket de agente."}{" "}
          La cuenta remota requiere una comprobación de acceso.
        </p>
        <p className="inventory-meta">{sshStatus[ssh.configStatus]}</p>
        <p>
          Archivos de clave predeterminados ·{" "}
          <KeyCounts keys={ssh.defaultKeys} />
        </p>
        {ssh.aliases.length > 0 && (
          <ul className="inspector-remotes">
            {ssh.aliases.map((alias) => (
              <li key={alias.name}>
                <strong>{alias.name}</strong>
                {alias.host && <> · {alias.host}</>}
                <div className="inventory-meta">
                  Archivos declarados · <KeyCounts keys={alias.keys} />
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="inventory-section">
        <h3>Acceso HTTPS</h3>
        <p>
          {https.helpers
            .map((item) => helpers[item] ?? helpers.custom)
            .join(", ") || "Sin helper general configurado."}
        </p>
        {https.scopedHelpers && (
          <p className="inventory-meta">
            Hay helpers específicos por URL. Su selección puede variar por
            remoto.
          </p>
        )}
        {https.askpassConfigured && (
          <p className="inventory-meta">
            El entorno o Git declara un mecanismo de solicitud de credenciales.
          </p>
        )}
      </div>
      <div className="inventory-section">
        <h3>Firma de commits</h3>
        <p>
          {signing.enabled === null
            ? "Preferencia de firma inválida."
            : signing.enabled
              ? "Firma automática activada."
              : "Firma automática desactivada."}{" "}
          {signingFormats[signing.format] ?? signingFormats.unknown}
        </p>
        <p className="inventory-meta">
          {keyReference[signing.keyReference] ?? keyReference.default}
          {signing.keyFile && <> · {keyFile[signing.keyFile]}</>}
        </p>
        <p className="inventory-meta">
          {signing.customProgram
            ? "Programa de firma personalizado configurado."
            : signing.defaultProgramAvailable
              ? "Herramienta predeterminada detectada en PATH."
              : "Herramienta predeterminada sin detectar en PATH."}
        </p>
        {signing.format === "ssh" &&
          !signing.customProgram &&
          ["public", "file"].includes(signing.keyReference) && (
            <ProbeControl id={id} kind="signing" />
          )}
      </div>
      {ordinary && <HookControl id={id} />}
    </>
  );
}
