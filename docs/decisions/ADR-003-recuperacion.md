# ADR-003 · Configuración recuperable

**Estado** · Aplicación inicial y restauración verificadas con archivos temporales en Windows y Linux. La validación de un corte de energía y los escenarios de publicación quedan pendientes.

Vincular el consentimiento al plan y sus huellas. Generar los perfiles inmutables, adquirir un bloqueo de aplicación y tomar los archivos `<destino>.lock` de Git de forma exclusiva y en orden estable. Releer destino, permisos, dependencias y huellas bajo bloqueo. Usar el reemplazo por destino en el mismo filesystem. Tomar el lock nuevamente al compensar una escritura y comprobar de nuevo el estado. Un lock ajeno provoca espera acotada o un error recuperable. La coordinación cubre escritores que respetan el lock y las precondiciones reducen el riesgo frente a editores externos. [Protocolo de locks de Git](https://github.com/git/git/blob/v2.39.0/lockfile.h)

Antes de publicar cada cambio, guardar en un journal durable la huella y existencia previa, identidad del destino, huella del candidato y referencia al respaldo. Cifrar y verificar los respaldos externos antes de escribir. Proteger la clave con el almacén del SO y detener el plan si el respaldo o su recuperación no están disponibles. Conservar permisos y atributos aplicables. En Windows, validar semántica de reemplazo, ACL y persistencia con APIs nativas. En POSIX, validar escritura y sincronización del archivo y del directorio padre. [Reemplazo en Windows](https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-replacefilew) · [Seguridad](../security/SECURITY.md)

| Estado observado al recuperar | Acción |
| --- | --- |
| Operación confirmada de forma durable | Mantener el resultado y reconciliar el registro de actividad. Tratar cambios posteriores como modificaciones externas. |
| Operación sin confirmar y destino en su estado previo | Registrar que el destino ya está restaurado o nunca se publicó. |
| Operación sin confirmar y destino verificable en el estado escrito por GitPilot | Restaurar el respaldo bajo lock o retirar el archivo creado si estaba ausente. Verificar con Git. |
| Operación sin confirmar y cambio ajeno, destino sustituido, respaldo ilegible o estado ambiguo | Mantener respaldo y journal, señalar recuperación pendiente y bloquear escrituras afectadas. |

Guardar la marca de confirmación y la revisión aplicada en una misma transacción de SQLite después de verificar con Git. Para una operación sin esa marca, revertir de forma conservadora en orden inverso. La prueba de fase 0 valida contención de Git y restauración con archivos sintéticos. Las garantías de durabilidad, cifrado, permisos y recuperación ante cierre se verifican al implementar el adaptador.
