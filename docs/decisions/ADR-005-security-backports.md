# ADR-005 · Backports de dependencias para 1.1.0

Estado: implementación en validación. Fecha: 2026-11-12.

## Motivo

SQLx 0.8.6 incluye SQLite vulnerable; SQLx 0.9.0 todavía limita libsqlite3-sys a <0.38.0. Tauri 2 usa GTK3 con glib 0.18.5, cuyo iterador de strings contiene el defecto de RUSTSEC-2024-0429. Cambiar solo el número de versión de GLib rompería la compatibilidad con GTK3. No migramos a Tauri 3 alpha para esta corrección.

## Decisión y alcance

- Actualizar SQLx a 0.9.0 con sus features de SQLite, sin habilitar RSA/MySQL. El nuevo lockfile elimina RSA. Sustituir las diez consultas SQL construidas con constantes por literales: SQLx 0.9 exige marcar la seguridad de SQL dinámico y aquí no hace falta SQL dinámico.
- Conservar el código de sqlx-sqlite 0.9.0 y fijar únicamente su dependencia libsqlite3-sys a 0.39.1, que incluye SQLite 3.54.1 corregido. La API FFI usada por el controlador se compila y se comprueba con pruebas de almacenamiento, migración, WAL y recuperación en los dos sistemas. No cambiar los checksums ni los SQL de migraciones existentes.
- Aplicar en glib 0.18.5 el cambio upstream de `let p`/`&p` a `let mut p`/`&mut p` en VariantStrIter::impl_get. Conservar el resto de la API y validar los cinco métodos afectados en una prueba optimizada de Linux.
- Conservar las fuentes de glib-macros 0.18.5 y gtk3-macros 0.18.2, sustituyendo proc-macro-error 1 por proc-macro-error3 3.1.1 con soporte syn2. Adaptar los imports al nombre real de la nueva dependencia, incluido el código generado por su atributo. No usar proc-macro-error2, cuyo repositorio está archivado.

Las cuatro copias viven en `apps/desktop/src-tauri/vendor`, con licencias MIT/Apache originales. `sources.json` registra el SHA-256 de cada archivo y de los archivos `.crate` originales de crates.io. `patches/` contiene las diferencias respecto de esos paquetes. Las copias se toman del registro de Cargo; no se descarga código desde forks sin fijar.

## Auditoría y límites

Cargo audit no relaciona automáticamente crates de tipo path con sus avisos de crates.io. Un resultado vacío no demuestra que el backport sea correcto. CI usa `scripts/audit-dependencies.py`: primero exige con `check-vendored-dependencies.py` las fuentes revisadas completas, selección de los parches en manifiesto/lockfile, SQLite 0.39.1 y ausencia de RSA/proc-macro-error originales. Después audita una copia temporal del lockfile que vuelve a identificar los cuatro paquetes como sus versiones de crates.io. Así las alertas futuras de las versiones originales siguen visibles. Solo clasifica RUSTSEC-2024-0429 como corregida cuando los hashes del backport completo coinciden; cualquier otro hallazgo bloquea CI. No hay lista global de alertas ignoradas. La revisión humana de los parches y las pruebas de regresión siguen siendo necesarias.

Las licencias de las dependencias conservan sus términos; la licencia del producto no las sustituye. Cualquier modificación de una copia exige revisar el diff upstream, actualizar el inventario de hashes y repetir los controles afectados. La prueba de SQLite valida la versión en ejecución, no solo la versión declarada por Cargo.

## Mantenimiento y retirada

Responsable: mantenedor de GitPilot. Revisar upstream antes de cada release y, como máximo, el 2026-12-12. Retirar cada copia en cuanto una versión compatible publicada incluya su corrección, eliminando también su patch, inventario y verificación específica. Repetir auditorías, migraciones y pruebas Windows/Linux. Si una copia deja de poder mantenerse o validar, bloquear la release; no cambiar su versión artificialmente para evitar un aviso.

## Fuentes

- [SQLx 0.9.0](https://github.com/transact-rs/sqlx/releases/tag/v0.9.0).
- [Aviso SQLite](https://rustsec.org/advisories/RUSTSEC-2026-0459.html).
- [Corrección upstream de GLib](https://github.com/gtk-rs/gtk-rs-core/pull/1343).
- [proc-macro-error3](https://github.com/gamma0987/proc-macro-error3).
