# Autenticación remota y firma

Alcance posterior a v0.1.0. Avanzar con [ADR-004](../decisions/ADR-004-autenticacion.md) revisado.

| Capacidad | Comportamiento previsto |
| --- | --- |
| SSH | Referenciar claves existentes y alias, detectar permisos y archivos faltantes, inspeccionar configuración y probar hosts de forma explícita. Validar host, opciones y cliente SSH. Respetar la comprobación de host y tratar conexiones de prueba como operaciones de red. Una edición de `~/.ssh/config` requiere diff, respaldo, confirmación, prueba y recuperación. |
| HTTPS | Detectar el mecanismo de credenciales del entorno y guiar al usuario con Git Credential Manager u otro helper compatible. Mantener las credenciales bajo ese mecanismo. Las pruebas activas requieren acción explícita y remotos permitidos. |
| Firma | Configurar referencias a claves de firma SSH/OpenPGP por separado. Diagnosticar formato, disponibilidad y configuración efectiva. No generar ni copiar claves privadas automáticamente. |
| GitHub OAuth | Conectar varias cuentas en el navegador, identificar por ID remoto y guardar tokens en el almacén del SO. Consultar perfil, organizaciones, repositorios autorizados, PR e issues. Gestionar expiración, cancelación, revocación y límites de API. |
| Inspector y pre-commit | Mostrar la cuenta autenticada solo cuando sea comprobable. La advertencia en pre-commit se instala voluntariamente y debe coexistir con hooks del usuario. El bloqueo exige habilitación explícita y la desinstalación restaura el estado autorizado. |

El flujo de escritorio candidato es Device Flow con un identificador público. El estudio comparará GitHub App y OAuth App por permisos mínimos, repositorios privados, organizaciones y revocación. Implementar polling que respete intervalos y expiración, y gestionar cancelación. Considerar autorización con PKCE solo si el flujo elegido funciona con cliente público y sin secretos embebidos. [Flujos de autorización de GitHub](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)

Usar almacenes seguros por plataforma, con Windows Credential Manager y Linux Secret Service como candidatos. Si no están disponibles, explicar cómo habilitarlos y evitar persistir secretos. Al desconectar, retirar el secreto local y exponer si la revocación remota quedó pendiente. El acceso al API de GitHub, la clave usada para Git remoto y la identidad de commit pueden pertenecer a perfiles diferentes.

## Inspector local

- El Inspector revalida la ruta y el Git dir del repositorio registrado. Lee el índice sin bloqueos opcionales ni monitores externos. Para contar cambios sin preparar, Git compara el árbol con el índice original desde un Git dir temporal con una lista cerrada de ajustes. Los filtros declarados en archivos rastreados, el índice no compatible o un resultado fuera del presupuesto dejan ese conteo sin verificar. [Estado de Git](https://git-scm.com/docs/git-status)
- Los destinos de lectura y envío provienen de Git y tienen en cuenta sus reglas de reescritura. A la interfaz llegan el nombre del remoto, el transporte y el host. El usuario, contraseña, ruta, consulta y comando de transporte permanecen fuera de la respuesta. [Remotos de Git](https://git-scm.com/docs/git-remote)
- En SSH se comprueban el cliente en PATH, la presencia de un socket anunciado y los archivos predeterminados o referencias simples dentro de `~/.ssh`. Los alias literales se muestran como indicios del archivo de usuario. Los bloques condicionales, las inclusiones y los proxies requieren revisión manual. La lectura de archivos de clave se limita a sus metadatos y los permisos Unix se muestran como indicios. [Configuración de OpenSSH](https://man.openbsd.org/ssh_config)
- Para HTTPS se clasifican los helpers generales, se respeta el reinicio de la lista y se indica cuando hay reglas por URL. Los argumentos de los helpers y las credenciales permanecen en el mecanismo configurado. Para firma se muestra la preferencia, formato y tipo de referencia. Una referencia SSH a un archivo simple dentro del directorio local `.ssh` admite comprobación de presencia. [Configuración de Git](https://git-scm.com/docs/git-config)

Las cuentas remotas y la posibilidad efectiva de autenticar o firmar requieren una prueba explícita. El Inspector local prescinde de conexiones, ejecución de helpers, consultas a agentes, programas de firma y escritura de configuración.

## Comprobaciones explícitas

La confirmación liga el repositorio, el destino o la clave pública y el tipo de prueba. SSH y HTTPS consultan `HEAD` con `git ls-remote` en un contexto temporal sin la configuración del repositorio. Se admiten destinos directos de lectura en GitHub, GitLab y Bitbucket. HTTPS usa TLS verificado y lectura anónima, sin redirecciones. SSH usa OpenSSH directo en modo no interactivo y exige un host conocido. Los resultados acreditan la respuesta del remoto al intento de lectura. Los alias, puertos alternativos, helpers de credenciales y comprobaciones de cuenta requieren otra modalidad. [Lectura de referencias remotas](https://git-scm.com/docs/git-ls-remote)

La firma SSH usa una clave pública configurada directamente o un archivo `.pub` simple del directorio SSH de Git. Se firma un desafío temporal con el agente y se comprueba su integridad con OpenSSH. La operación carece de efectos sobre el historial y elimina sus archivos temporales al terminar. [Firma con ssh-keygen](https://man.openbsd.org/ssh-keygen)

## Hook pre-commit

La instalación exige un repositorio ordinario, una identidad esperada y la ruta de hooks predeterminada libre. GitPilot conserva los hooks existentes y los contextos con `core.hooksPath` personalizado. El modo inicial emite un aviso y el bloqueo exige selección y confirmación. La identidad revisada queda en el script hasta retirarlo e instalar una nueva versión. La retirada usa el journal y comprueba la identidad y huella del archivo. Ante una edición externa, Actividad presenta el destino como recuperación pendiente. [Hooks de Git](https://git-scm.com/docs/githooks)
