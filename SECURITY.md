# Política de seguridad · Security policy

## Reportar una vulnerabilidad

Usa **Report a vulnerability** en GitHub para compartir los pasos de reproducción, la versión de GitPilot, el sistema operativo, la versión de Git y el impacto. Usa datos ficticios o redactados. Conserva las credenciales, las claves privadas y los respaldos en sus almacenes locales. [Abrir reporte privado](https://github.com/JhosepAC/gitpilot/security/advisories/new)

El mantenedor revisa el reporte en el aviso privado y coordina los pasos de reproducción, la corrección y su divulgación. La revisión se centra en la versión estable más reciente y los cambios integrados en `main`.

GitPilot guarda perfiles, reglas y actividad en SQLite en el directorio local de datos. Las credenciales de GitHub y las claves de respaldo usan Windows Credential Manager o Secret Service. Usa **Actividad** para revisar y restaurar cambios Git verificados. [Datos y respaldos](docs/architecture/DATABASE.md) · [Recuperación](docs/architecture/GIT_CONFIGURATION.md) · [Modelo de amenazas](docs/security/SECURITY.md)

## Reporting in English

Use GitHub's **Report a vulnerability** form. Include the app, OS, and Git versions, the impact, and steps to reproduce using synthetic or redacted data. The maintainer coordinates fixes and disclosure in the private advisory. Reports are reviewed against the latest stable release and `main`. [Open a private report](https://github.com/JhosepAC/gitpilot/security/advisories/new)
