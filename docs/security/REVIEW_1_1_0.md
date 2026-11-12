# Revisión de seguridad del candidato 1.1.0

Fecha: 2026-11-12. Estado: **publicación bloqueada por dependencias Rust**.

## Alcance y resultados

- Revisión de los cambios de interfaz, rutas representadas como texto React, capacidades Tauri y generación de comandos SSH. Sin nuevas capacidades privilegiadas en el rediseño. Se mantienen escapes de shell, validación de campos, confirmación ligada al plan, respaldo y verificación.
- Búsqueda acotada de claves privadas y tokens en apps, docs y workflows: solo una cadena sintética de prueba de rechazo, sin material privado detectado. No equivale a una auditoría completa del historial.
- `npm audit`: inicialmente 10 alertas derivadas de minimatch 10.2.6; tras fijar 11.0.1 con override, cero vulnerabilidades conocidas. ESLint, TypeScript, formato, compilación y las cuatro pruebas Vitest pasaron con el override.
- `cargo audit` 0.23.0: dos vulnerabilidades en el lockfile y dos advertencias. Base RustSec revisada al 2026-11-12. No se ignoran alertas.

## Bloqueos pendientes

| Alerta | Evaluación y acción necesaria |
| --- | --- |
| RUSTSEC-2026-0459 / CVE-2026-119375 | libsqlite3-sys 0.30.1, activo por SQLx 0.8.6 y SQLite incluido. Una pareja de archivos DB/WAL manipulada puede exponer memoria del proceso. Corregido en libsqlite3-sys >=0.39.0. SQLx 0.9.0 admite <0.38.0, por lo que actualizar solo SQLx no elimina la alerta. Se necesita un controlador compatible con SQLite corregido o un parche del controlador revisado y validado en ambas plataformas. |
| RUSTSEC-2023-0071 | rsa 0.9.10 aparece en Cargo.lock; `cargo tree -i rsa` no encuentra consumidores en el destino Windows activo. No se acreditó alcance en todos los destinos. Resolver el lockfile/controlador y repetir auditoría, sin ocultar la alerta por su inactividad en Windows. |
| RUSTSEC-2024-0429 | Advertencia de unsoundness en glib 0.18.5 del ecosistema GTK de Linux; revisar actualización compatible de la cadena Tauri/GTK antes de distribuir Linux. |
| RUSTSEC-2024-0370 | proc-macro-error sin mantenimiento; dependencia de compilación que requiere seguimiento y sustitución por upstream. |

Referencias: https://rustsec.org/advisories/RUSTSEC-2026-0459.html, https://rustsec.org/advisories/RUSTSEC-2023-0071.html, https://rustsec.org/advisories/RUSTSEC-2024-0429.html, https://rustsec.org/advisories/RUSTSEC-2024-0370.html, https://github.com/advisories/GHSA-g3hv-75wg-m249.

Desktop CI ejecuta las auditorías de npm y Cargo. Release candidate exige Desktop CI aprobado para el SHA de main, por lo que estas alertas impiden la distribución oficial. Un instalador local generado es exclusivamente un candidato; no se ha certificado libre de vulnerabilidades ni probado su instalación o actualización en esta revisión.

## Validación funcional

Windows: formato Rust, Clippy sin advertencias y pruebas Rust correctos (40 aprobadas, 5 omitidas por entorno especial). Frontend: formato, lint, comprobación de tipos, compilación y 4 pruebas correctos. El test de asignación usa Git y configuración temporales y comprueba aislamiento SSH, sustitución de claves, autor y restauración. No se repitió validación Linux ni las pruebas que necesitan almacenes de credenciales aislados. La revisión visual del rediseño no se completó por interferencia de entrada en el navegador.
