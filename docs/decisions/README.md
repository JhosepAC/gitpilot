# Decisiones de arquitectura

Los ADR iniciales registran la dirección de trabajo. Resolver las decisiones condicionadas con evidencia antes de iniciar la fase que dependa de ellas.

| ADR | Decisión | Estado |
| --- | --- | --- |
| [001](ADR-001-escritorio.md) | Monolito modular local-first con Tauri, Rust y SQLite | Propuesto |
| [002](ADR-002-git-nativo.md) | Git CLI e inclusiones acotadas a Git dirs ordinarios | Adoptado para Windows y Linux |
| [003](ADR-003-recuperacion.md) | Locks de Git, respaldo cifrado y recuperación por journal | Protocolo definido, adaptador por validar |
| [004](ADR-004-autenticacion.md) | Separar OAuth, acceso remoto y firma, con secretos en el SO | Propuesto para después del MVP |

| Decisión pendiente | Momento y evidencia |
| --- | --- |
| Reemplazo nativo, respaldo cifrado y durabilidad | Antes de habilitar escrituras. Probar permisos, almacén seguro, acceso al respaldo y fallos por etapa. |
| Aplicación de GitHub y permisos | Antes de fase 6. Validar Device Flow, acceso privado/organizaciones y revocación. |
| Licencia, plataformas exactas de distribución y firma | Antes de publicar los instaladores de cada hito. Registrar versión mínima de SO, arquitectura, mecanismo y responsable. |
