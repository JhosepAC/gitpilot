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
