# ADR-004 · Identidades y credenciales independientes

**Estado** · Adoptado para la fase 6. La autorización real y los almacenes de tokens requieren la validación de esta fase.

Usar el almacén seguro del SO para secretos de GitPilot, referencias para claves existentes y los mecanismos nativos de Git para SSH y HTTPS. La firma, OAuth de API y autenticación del remoto son capacidades independientes.

## GitHub

Registrar una **GitHub App** con permisos de repositorio `Metadata`, `Issues` y `Pull requests` en lectura y selección explícita de repositorios durante la instalación. Esta elección permite limitar el acceso al alcance del producto y a los recursos compartidos por la instalación y el usuario. [Comparación de tipos de aplicación](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps)

Se evaluaron OAuth App y el flujo web con PKCE. OAuth App concede scopes más amplios para las consultas privadas previstas. La documentación del canje web para GitHub Apps sigue requiriendo `client_secret` junto al soporte de PKCE. Device Flow cubre el cliente público de escritorio con los permisos de una GitHub App. [Elección de aplicación](https://docs.github.com/en/apps/creating-github-apps/about-creating-github-apps/deciding-when-to-build-a-github-app), [canje del flujo web](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-web-application-flow-to-generate-a-user-access-token)

Usar **Device Flow** para la autorización de escritorio. El cliente usa el `Client ID` público, solicita tokens con vencimiento y mantiene los códigos de dispositivo dentro de Rust. La autorización se completa en el navegador del sistema. GitHub documenta este flujo para aplicaciones de escritorio. [Tokens de usuario y Device Flow](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token)

La renovación de tokens originados mediante Device Flow admite el identificador público de aplicación y el refresh token. La distribución de escritorio puede implementar esa renovación con los secretos propios de cada cuenta en el almacén del SO. La rotación exige serialización y recuperación ante respuestas o escrituras interrumpidas. [Renovación de tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens)

La desconexión retira los secretos locales y ofrece abrir la configuración de autorizaciones de GitHub para revocar el permiso. La API de revocación de autorizaciones exige el secreto de la aplicación y corresponde a un servicio confidencial. La interfaz registra la acción remota pendiente hasta que el usuario la atienda. [Revocación de autorizaciones](https://docs.github.com/en/rest/apps/oauth-applications#delete-an-app-authorization)

SQLite conserva referencias y metadatos. El backend verifica la cuenta mediante su ID remoto. GitHub puede mostrar una cuenta distinta de la que accede por SSH. La [especificación de GitHub](../architecture/GITHUB.md) fija los contratos, la preparación del registro y la evidencia necesaria antes de publicar la integración.
