# ADR-004 · Identidades y credenciales independientes

**Estado** · Propuesto para después del MVP.

Usar el almacén seguro del SO para secretos de GitPilot, referencias para claves existentes y los mecanismos nativos de Git para SSH y HTTPS. Evaluar Device Flow para GitHub con un cliente público y permisos mínimos. La firma, OAuth de API y autenticación del remoto son capacidades independientes.

Consecuencias. SQLite conserva referencias y metadatos. GitHub puede mostrar una cuenta distinta de la que accede por SSH. Validar almacenes, permisos, revocación y compatibilidad con organizaciones antes de publicar la integración. [Autenticación](../security/AUTHENTICATION.md)
