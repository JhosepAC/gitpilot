# Historial de cambios

## 1.1.0

- Autenticación remota SSH por identidad: configura `core.sshCommand` con la clave privada elegida, activado automáticamente vía `includeIf gitdir:` al asignar un directorio.
- Firma de commits por identidad: soporte para claves SSH y OpenPGP (`commit.gpgsign`, `gpg.format`, `user.signingkey`) sin tocar la configuración global.
- Aplicación automática de la configuración Git al guardar la identidad y al asignar un directorio, sin pasos de confirmación adicionales.
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
