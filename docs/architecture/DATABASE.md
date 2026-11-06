# Persistencia local

## Propiedad de los datos

SQLite guarda la intención del usuario, inventario, estado de operaciones y metadatos. Git y sus archivos determinan el resultado efectivo. Los archivos generados, journal y respaldos se guardan en rutas privadas de aplicación elegidas por SO. Los archivos que Git necesita leer permanecen disponibles después de cerrar GitPilot. Caché, logs y respaldos tienen ciclos de vida separados.

## Modelo del MVP

| Tabla | Campos y restricciones |
| --- | --- |
| `identities` | ID estable, etiqueta, nombre Git, correo, revisión y marcas de tiempo. Correo admite duplicados. |
| `directories` | ID, ruta mostrada, ruta canónica/identificador de filesystem, estado y revisión. Un registro por raíz reconocida. |
| `directory_exclusions` | Raíz, ruta relativa normalizada y estado. Clave única por raíz y ruta. La exclusión delimita el escaneo. |
| `repositories` | ID, raíz de trabajo visible y canónica, Git dir efectivo, Git dir común, tipo, estado y última observación. Los worktrees conservan una fila por contexto de trabajo. Un repositorio puede aparecer bajo varias raíces. |
| `identity_rules` | Identidad, objetivo único, revisión. El objetivo es una raíz/directorio registrado o un repositorio. Restricción XOR y unicidad por tipo de objetivo activo. Eliminar una identidad referenciada exige resolver sus asignaciones. |
| `app_settings` | Preferencias tipadas, tema, límites de escaneo y referencia opcional a una sola identidad predeterminada. |
| `config_operations` | ID, operación relacionada si restaura, versión del plan, estado, revisión de intención, huella y marcas de tiempo. El journal durable registra etapas y permite recuperar operaciones incompletas. |
| `config_backups` | Operación, destino, referencia privada al respaldo, existencia previa, huellas anterior/posterior y retención. Restauración conserva permisos y atributos requeridos. |
| `audit_logs` | Acción, referencias, resultado, código de error filtrado, fecha. Guarda la decisión de conservar con huella contextual para detectar cambios posteriores. |

Activar claves foráneas en cada conexión. Usar migraciones versionadas, transacciones para cambios internos e índices para reglas por destino, repositorios por ubicación y auditoría por fecha. Registrar rutas sin perder información necesaria para operar en el SO. Ante una representación que no pueda serializarse o usarse con seguridad en Git, ofrecer diagnóstico y excluirla de escrituras.

El plan liga revisiones y huellas de todos los archivos relevantes, incluidas las dependencias externas que afectan la verificación. Confirmar una operación actualiza el estado aplicado en SQLite solo después de verificar con Git. Conservar el snapshot necesario para interpretar un estado aplicado incluso si el perfil ya tiene un borrador distinto. Recuperar o revertir antes de admitir otra escritura tras una interrupción.

## Extensiones posteriores

| Tabla o relación | Uso |
| --- | --- |
| `signing_keys` | Referencias públicas o selectores de firma. La clave privada sigue bajo la herramienta externa. |
| `ssh_keys` y perfiles de autenticación | Referencias a claves existentes, hosts y alias. Asociaciones por remoto o contexto, independientes del perfil Git. |
| `auth_accounts` | Proveedor, ID estable del usuario remoto, nombre visible y referencia al almacén seguro. Una cuenta puede relacionarse con varios remotos y perfiles. |
| `repository_remotes` | Repositorio, nombre, endpoint sin credenciales y asociaciones. Tratar la caché como derivada de Git. |
| `github_installations` | Metadatos de instalaciones y organizaciones si el tipo de aplicación elegido los requiere. |

La exportación futura serializa perfiles, reglas y preferencias seleccionadas. Validar el formato y las rutas al importar y previsualizar cualquier cambio Git. Tokens, claves privadas, journal y respaldos quedan fuera de la exportación.
