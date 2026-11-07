# Persistencia local

## Propiedad de los datos

SQLite guarda la intención del usuario, inventario, estado de operaciones y metadatos. Git y sus archivos determinan el resultado efectivo. Los archivos generados, journal y respaldos se guardan en rutas privadas de aplicación elegidas por SO. Los archivos que Git necesita leer permanecen disponibles después de cerrar GitPilot. Caché, logs y respaldos tienen ciclos de vida separados.

## Modelo del MVP

| Tabla | Campos y restricciones |
| --- | --- |
| `identities` | ID estable, etiqueta, nombre Git, correo, revisión y marcas de tiempo. Correo admite duplicados. Un índice parcial garantiza como máximo un perfil predeterminado. |
| `directories` | ID, ruta canónica, revisión y estado de escaneo. Un registro por raíz reconocida. |
| `directory_exclusions` | Raíz y ruta canónica de la subcarpeta. Clave única por raíz y ruta. La exclusión delimita el escaneo. |
| `repositories` | ID, ruta canónica, Git dir efectivo, Git dir común, tipo y revisión. Los worktrees conservan una fila por contexto de trabajo. |
| `directory_repositories` | Pertenencia y presencia de cada repositorio por raíz. Un repositorio puede aparecer bajo varias raíces y compartir su excepción. |
| `identity_rules` | Identidad y objetivo único. El objetivo es una raíz registrada o un repositorio y su revisión protege las actualizaciones de la regla. Restricción XOR y unicidad por tipo de objetivo. Eliminar una identidad referenciada exige retirar o cambiar sus asignaciones. |
| `app_settings` | Preferencias tipadas, tema y límites de escaneo. |
| `config_operations` | ID, operación relacionada si restaura, versión del plan, estado, revisión de intención, huella y marcas de tiempo. El journal durable registra etapas y permite recuperar operaciones incompletas. |
| `config_backups` | Operación, destino, referencia privada al respaldo, existencia previa, huellas anterior/posterior y retención. Restauración conserva permisos y atributos requeridos. |
| `audit_logs` | Acción, referencias, resultado, código de error filtrado, fecha. Guarda la decisión de conservar con huella contextual para detectar cambios posteriores. |

Activar claves foráneas en cada conexión. Usar migraciones versionadas, transacciones para cambios internos e índices para reglas por destino, repositorios por ubicación y auditoría por fecha. Registrar rutas sin perder información necesaria para operar en el SO. Ante una representación que no pueda serializarse o usarse con seguridad en Git, ofrecer diagnóstico y excluirla de escrituras.

La revisión del perfil se comprueba al editarlo, duplicarlo, cambiar su estado predeterminado o eliminarlo. La sustitución del predeterminado se hace en una transacción. Antes de eliminarlo hay que retirar esa preferencia o trasladarla a otro perfil. Una clave foránea impide eliminar perfiles asignados.

Un escaneo completo actualiza la presencia de los repositorios de su raíz. Conserva las excepciones de los repositorios ausentes para poder retirarlas desde GitPilot. Un escaneo parcial agrega o actualiza los resultados verificados y conserva las pertenencias anteriores. Guardar un escaneo requiere que la revisión de la raíz coincida con la capturada al inicio.

`app_settings` guarda las preferencias de escaneo en la clave `inventory.scan`. Una base anterior usa 12 segundos de recorrido, 12 000 entradas, profundidad 24 y 200 repositorios. La actualización valida los intervalos y compara los valores que se revisaron al abrir el formulario para detectar cambios concurrentes.

El plan liga revisiones y huellas de todos los archivos relevantes, incluidas las dependencias externas que afectan la verificación. Confirmar una operación actualiza el estado aplicado en SQLite solo después de verificar con Git. Conservar el snapshot necesario para interpretar un estado aplicado incluso si el perfil ya tiene un borrador distinto. Recuperar o revertir antes de admitir otra escritura tras una interrupción.

## Cuentas de GitHub

La migración `202611070002_github_accounts.sql` convierte el slot de la primera cuenta en una fila identificada por Client ID e ID remoto. Conserva el login, las fechas y la referencia al almacén seguro. `github_credentials` registra a qué cuenta pertenece cada referencia en preparación, activa o pendiente de retirada. Los tokens y sus vencimientos se guardan en el almacén del SO.

La renovación registra una referencia pendiente antes de solicitar credenciales y la conserva mientras se verifica la identidad devuelta por GitHub. Tras verificarla, una transacción activa la nueva referencia. La reconciliación elimina referencias huérfanas bajo un lock compartido entre instancias. `github_revocations` conserva el ID y el login tras la retirada local hasta que el usuario da por revisada la autorización en GitHub. [Contrato de GitHub](GITHUB.md)

La migración `202611070003_github_remote_links.sql` registra un vínculo por repositorio local y nombre de remoto. Guarda los IDs de la cuenta, instalación y repositorio autorizado, la huella del destino efectivo de lectura y la versión del vínculo. La versión protege el reemplazo y la retirada frente a cambios concurrentes. La eliminación del repositorio local o la desconexión de la cuenta elimina sus vínculos mediante las claves foráneas.

## Portabilidad

El JSON v1 contiene perfiles de autor, asignaciones de identidad por ruta y las preferencias de idioma (`ui.language`) y escaneo (`inventory.scan`). Los identificadores de perfil relacionan las asignaciones dentro del archivo. El límite es 1 MiB. La exportación lee una instantánea de SQLite y guarda el archivo elegido en el selector nativo. Si el destino ya existe, solicita elegir otro nombre. Tokens, claves privadas, datos de cuentas, inventario, exclusiones, actividad y respaldos se conservan en sus almacenes respectivos.

La importación valida versión, perfiles, referencias, rutas absolutas y límites de escaneo. La revisión muestra los perfiles que se añadirán o reutilizarán, las asignaciones y las preferencias que cambiarán. Se conservan el perfil predeterminado y las asignaciones locales existentes. Una ruta del archivo debe coincidir con un directorio registrado o un repositorio presente. La confirmación revalida la instantánea en una transacción SQLite. La aplicación a Git usa su propio flujo de revisión, respaldo, confirmación y verificación.

La migración `202611070004_portable_activity.sql` añade un registro local con tipo de transferencia, fecha y número de perfiles en el archivo. La importación registra el evento en su transacción y la exportación después de guardar. Actividad combina esas entradas con el historial de Git.

## Extensiones posteriores

| Tabla o relación | Uso |
| --- | --- |
| `signing_keys` | Referencias públicas o selectores de firma. La clave privada sigue bajo la herramienta externa. |
| `ssh_keys` y perfiles de autenticación | Referencias a claves existentes, hosts y alias. Asociaciones por remoto o contexto, independientes del perfil Git. |
| `auth_accounts` | Proveedor, ID estable del usuario remoto, nombre visible y referencia al almacén seguro. Una cuenta puede relacionarse con varios remotos y perfiles. |
| `repository_remotes` | Repositorio, nombre, endpoint sin credenciales y asociaciones. Tratar la caché como derivada de Git. |
| `github_installations` | Metadatos de instalaciones y organizaciones si el tipo de aplicación elegido los requiere. |
