# ADR-001 · Monolito modular de escritorio

**Estado** · Propuesto.

GitPilot necesita acceso local a Git, archivos y capacidades del SO. Usará Tauri 2, React/TypeScript/Vite y Rust, con persistencia SQLite mediante SQLx. El dominio queda separado del IPC, de Git y del almacenamiento. Los adaptadores por plataforma se incorporan donde existan diferencias reales.

Consecuencias. La aplicación puede gestionar identidades sin red y la persistencia se distribuye con el producto. El equipo debe validar compilación y webviews en Windows y Linux y mantener migraciones compatibles. [Arquitectura](../architecture/ARCHITECTURE.md)
