# Decisiones de arquitectura

Los ADR iniciales registran la dirección de trabajo. Resolver las decisiones condicionadas con evidencia antes de iniciar la fase que dependa de ellas.

| ADR | Decisión | Estado |
| --- | --- | --- |
| [001](ADR-001-escritorio.md) | Monolito modular local-first con Tauri, Rust y SQLite | Propuesto |
| [002](ADR-002-git-nativo.md) | Git CLI e inclusiones acotadas a Git dirs ordinarios | Adoptado para Windows y Linux |
| [003](ADR-003-recuperacion.md) | Locks de Git, respaldo cifrado y recuperación por journal | Adaptador inicial verificado en Windows y Linux |
| [004](ADR-004-autenticacion.md) | GitHub App con Device Flow, permisos de lectura y secretos en el SO | Adoptado para fase 6, validación de cuentas pendiente |

| Decisión pendiente | Momento y evidencia |
| --- | --- |
| Reemplazo nativo, respaldo cifrado y durabilidad | Antes de habilitar escrituras. Probar permisos, almacén seguro, acceso al respaldo y fallos por etapa. |
| Validación de la aplicación de GitHub | En fase 6. GitHub App y permisos definidos en ADR-004. Comprobar Device Flow, rotación, acceso privado y organizaciones y revocación. |
| Licencia, plataformas exactas de distribución y firma | Antes de publicar los instaladores de cada hito. Registrar versión mínima de SO, arquitectura, mecanismo y responsable. |
