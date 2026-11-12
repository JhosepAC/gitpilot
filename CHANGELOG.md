# Historial de cambios

## 1.1.0

- Conservación del propietario, grupo y ACL al reemplazar y recuperar archivos de configuración en Windows.

- SQLite actualizado a 3.54.1 mediante SQLx 0.9 y backport compatible; corrección de GLib y sustitución de macros sin mantenimiento, con auditoría e integridad verificadas.
- Pruebas adicionales de migración con WAL, acceso SSH aislado y firma SSH/OpenPGP.

- Autenticación remota SSH por identidad: configura `core.sshCommand` con la clave privada elegida, activado automáticamente vía `includeIf gitdir:` al asignar un directorio.
- Firma de commits por identidad: soporte para claves SSH y OpenPGP (`commit.gpgsign`, `gpg.format`, `user.signingkey`) sin tocar la configuración global.
- Asignación de perfiles con vista previa, confirmación, respaldo y verificación de la configuración efectiva, incluso ante valores globales o locales anteriores. Editar una identidad requiere reaplicarla desde Directorios.
- Aislamiento de autenticación SSH: sin clave propia se bloquea la autenticación por clave y se evita reutilizar claves o agentes de otro perfil. Firma opcional u obligatoria independiente de la autenticación.
- Navegación lateral vertical con iconos y etiquetas; directorios identificados por nombre con la ruta disponible en su detalle.
- Consulta de las rutas asignadas a cada identidad, con reglas de directorio y excepciones por repositorio.
- Corrección de GHSA-g3hv-75wg-m249 en la dependencia de desarrollo minimatch mediante la versión 11.0.1.
- Rediseño del editor de identidad con secciones separadas: autor del commit, autenticación remota (SSH) y firma de commits.
- Vista de detalle ampliada con tarjetas de estado para SSH y firma.
- Nueva migración de base de datos: columnas `signing_key`, `signing_format` y `ssh_key` en la tabla `identities`.

## 1.0.0

- Perfiles de autor, reglas de directorio y excepciones por repositorio, con diagnóstico de los valores efectivos de Git.
- Vista previa y aplicación confirmada de cambios Git, respaldos cifrados, actividad y restauración.
- Inspector local, comprobaciones separadas de SSH, HTTPS y firma, y hook de identidad optativo.
- Cuentas de GitHub con foto de perfil, instalaciones y repositorios autorizados, pull requests, issues y vínculos locales.
- Límites de escaneo configurables, progreso, cancelación, exportación JSON con versión, importación revisable y elección persistente de idioma para la navegación y las pantallas principales de portabilidad.
- Logo de GitPilot en la interfaz y los instaladores. Paquetes NSIS x64 y Debian amd64.
