# Revisión de seguridad del candidato 1.1.0

Fecha: 2026-11-12. Estado: **bloqueos de dependencias corregidos; distribución pendiente de CI y verificación final**.

## Correcciones

| Hallazgo inicial | Resolución |
| --- | --- |
| GHSA-g3hv-75wg-m249 | minimatch 11.0.1 fijado; npm audit sin alertas tras npm ci. |
| RUSTSEC-2026-0459 | SQLx 0.9.0 y backport mínimo de sqlx-sqlite para libsqlite3-sys 0.39.1. SQLite 3.54.1 comprobado en ejecución mediante prueba de migración con WAL en Windows y Linux. |
| RUSTSEC-2023-0071 | RSA retirado del lockfile con SQLx 0.9.0 sin activar su feature RSA. |
| RUSTSEC-2024-0429 | Corrección upstream de VariantStrIter aplicada a glib 0.18.5; cinco métodos comprobados en una prueba optimizada de Linux. |
| RUSTSEC-2024-0370 | proc-macro-error sustituido por proc-macro-error3 3.1.1 con syn2 en las macros de GLib/GTK. |

El [ADR-005](../decisions/ADR-005-security-backports.md) registra origen, licencias, modificaciones, mantenimiento y retirada de los cuatro backports. Sus fuentes y diferencias están versionadas; los avisos legales se incluyen en los instaladores.

## Auditoría y controles

`python scripts/audit-dependencies.py` verifica hashes y selección de cada copia y audita también las versiones originales de crates.io mediante un lockfile temporal. Solo reconoce el aviso conocido de GLib como corregido si las fuentes revisadas coinciden exactamente. Cualquier otro hallazgo bloquea CI. El resultado de cargo audit sobre dependencias path por sí solo no se usa como prueba de corrección.

La base RustSec comprobada es `af7b3ec33460b8e2de8d422e16d628518891a519` (2026-11-12), 1490 avisos. Sin hallazgos pendientes en la auditoría completa de los backports. Se comprobó que revertir la corrección de GLib o ejecutar el verificador con assertions desactivadas produce un fallo.

Gitleaks 8.30.1, obtenido de su release oficial y comprobado con SHA-256, analizó 73 commits y las fuentes vendorizadas sin detectar secretos. Actionlint 1.7.12 validó los workflows. Estos análisis no garantizan ausencia absoluta de vulnerabilidades.

## Validaciones realizadas y límites

- Windows: formato, Clippy, compilación, pruebas Rust habituales y las cinco pruebas antes omitidas de credenciales/rendimiento. Migración de base 1.0.0 con WAL, conservación de perfiles y reapertura correctas. Frontend: formato, lint, tipos y cuatro pruebas correctos.
- Linux, ejecución anterior a la omisión solicitada: 48 pruebas incluidas las omitidas, con una sesión Secret Service aislada; prueba optimizada de GLib; Clippy y compilación correctos. Una prueba adicional con SSH loopback verificó acceso exclusivo por la clave asignada y firmas SSH/OpenPGP independientes. No representa una validación completa de la distribución Debian.
- WebView2: menú vertical, carpetas homónimas con ruta accesible, asignaciones deduplicadas, rutas largas, navegación a Directorios y apertura/cancelación del selector nativo, con datos sintéticos y un identificador separado de QA.
- Revisión visual Linux y comprobaciones adicionales Linux omitidas por instrucción expresa del responsable el 2026-11-12. No se contabilizan como aprobadas. CI funcional de esta entrega se limita a Windows; la auditoría de dependencias cubre el lockfile completo.
- Permanecen advertencias de compilación heredadas en las copias de GLib/GTK; Clippy del proyecto pasa. Revisar upstream y retirar backports antes del 2026-12-12 o en la próxima release, lo que ocurra primero.

## Condiciones de distribución

El candidato definitivo debe construirse desde el SHA de main aprobado por CI, verificar sus sumas y su firma OpenPGP, y registrar la validación de instalación/actualización Windows antes de publicar. Las validaciones adicionales Linux quedan expresamente omitidas. Windows se distribuye sin Authenticode; las notas deben conservar esa información.


### Corrección adicional de permisos Windows

CI identificó dos cambios de metadatos: propietario/grupo del staging y duplicación de ACE heredadas al usar ReplaceFileW en Windows Server con DACL anteriores al modelo de herencia automática. El log de la prueba aislada registró tres ACE heredadas antes y seis (tres explícitas y tres heredadas) después. La reproducción local del grupo explícito falló antes de copiar el descriptor, pero la duplicación solo se observó en el servidor.

El motor copia propietario, grupo, DACL, protección de herencia y atributos al staging, verifica su huella y lo publica mediante MoveFileExW en el mismo directorio, sin fallback de copia entre volúmenes. Se mantienen la comprobación posterior y la recuperación por journal. No se cambia el algoritmo de huellas, para conservar compatibilidad con operaciones ya registradas. Archivos comprimidos, cifrados, sparse o con streams adicionales se rechazan antes de escribir, evitando perder metadatos que esta ruta no puede conservar. Creación y última modificación no forman parte del contrato de recuperación.

Las pruebas cubren grupo explícito, DACL heredada sin indicador AI y rechazo de streams alternativos sin modificar sus bytes. Las fixtures Git/HTTP de CI se limitan a dos hilos para evitar agotar sus presupuestos en el runner compartido. Pendiente de confirmar CI del SHA corregido.

Referencias: [ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew), [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw), [seguridad al renombrar archivos](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).
