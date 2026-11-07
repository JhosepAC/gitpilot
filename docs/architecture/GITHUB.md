# GitHub · fase 6

La entrega 6.1 implementa la autorización y persistencia de la primera cuenta sobre el [ADR-004](../decisions/ADR-004-autenticacion.md). Los apartados siguientes distinguen el contrato de la fase completa y el comportamiento disponible en esta entrega.

## Alcance

GitPilot conectará varias cuentas de GitHub.com, identificadas por su ID remoto. Permitirá consultar las instalaciones accesibles, sus repositorios y las listas de PR e issues, y asociar un repositorio autorizado con un remoto del inventario. Las organizaciones visibles serán las cuentas propietarias de instalaciones accesibles al usuario. [Instalaciones y repositorios accesibles](https://docs.github.com/en/rest/apps/installations#list-app-installations-accessible-to-the-user-access-token)

La asociación guardará metadatos en SQLite. Una modificación posterior de la URL remota tendrá su propia vista previa, confirmación, respaldo y recuperación. La identidad de commit, las claves de firma y la autenticación de Git conservan sus flujos específicos.

## Aplicación y permisos

Se usará una GitHub App y autorización de usuario por Device Flow. Los permisos se conceden al registrar e instalar la aplicación. El token opera dentro de la intersección del acceso de la aplicación y el usuario. [Permisos de los tokens de usuario](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#about-user-access-tokens)

| Permiso de repositorio | Nivel | Uso |
| --- | --- | --- |
| Metadata | Read-only | Identificación y repositorios accesibles por instalación. |
| Issues | Read-only | Listado de issues en un repositorio autorizado. |
| Pull requests | Read-only | Listado de PR en un repositorio autorizado. |

Los endpoints elegidos admiten tokens de usuario de GitHub Apps. Consultar `/user` permite verificar el ID de la cuenta. La lectura de issues y PR usa los permisos de la tabla. [Cuenta autenticada](https://docs.github.com/en/rest/users/users#get-the-authenticated-user), [issues](https://docs.github.com/en/rest/issues/issues#list-repository-issues), [PR](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests)

Los permisos de organización y cuenta quedan con su valor inicial. Los webhooks estarán desactivados en el registro de desarrollo. Se consultarán los recursos bajo demanda desde el cliente.

## Registro para desarrollo

El responsable realiza este registro en su cuenta o en una organización que administre. El Client ID resultante es público y será la entrada de la próxima entrega. [Registro de una GitHub App](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/registering-a-github-app)

1. Abre **Settings → Developer settings → GitHub Apps → New GitHub App**.
2. Elige un nombre disponible que identifique el entorno de desarrollo, por ejemplo `<responsable>-gitpilot-dev`. Usa la URL pública del producto o de la cuenta responsable como Homepage URL.
3. Activa **Enable Device Flow**. Mantén **User-to-server token expiration** activado. El callback puede quedar vacío para este flujo.
4. Desactiva **Active** en Webhook y configura los tres permisos de repositorio de la tabla en lectura. Conserva los valores iniciales del resto de permisos.
5. Empieza con **Only on this account** para el ensayo personal. La prueba posterior con una organización requiere permitir esa instalación y obtener la aprobación de su administrador.
6. Guarda el registro y copia el **Client ID** de su página de configuración. El App ID numérico identifica otro recurso.
7. Instala la aplicación en la cuenta de prueba con **Only select repositories**. Selecciona un repositorio privado temporal con un issue y una PR de prueba.

El responsable conservará las credenciales administrativas del registro. GitPilot usará el Client ID público y los tokens individuales obtenidos mediante autorización. Los registros de desarrollo y distribución tendrán identificadores separados.

El registro de desarrollo actual pertenece a `JhosepAC`, con App ID `5652387` y Client ID público `Iv23libwtluOIgGw4BuU`. El backend usa este Client ID para iniciar Device Flow. El repositorio privado `JhosepAC/test-gitpilot` se reserva para comprobar recursos autorizados en 6.3.

## Autorización de escritorio

El backend mantendrá una sesión con ID aleatorio, código de dispositivo en memoria protegida, plazo monotónico, intervalo e indicador de petición activa. El frontend recibirá el ID, el código que debe introducir el usuario, la URL de verificación permitida y el tiempo restante. Las respuestas tardías deberán corresponder a la sesión activa antes de guardar una cuenta.

| Paso | Operación y resultado |
| --- | --- |
| Iniciar | `POST https://github.com/login/device/code` con el Client ID público y `scope=offline_access`. Validar la respuesta antes de mostrar el código. |
| Autorizar | Mostrar el código y abrir `https://github.com/login/device` en el navegador del sistema mediante una acción del usuario. |
| Consultar | `POST https://github.com/login/oauth/access_token` con el Client ID, el código de dispositivo y el grant de dispositivo. |
| Verificar | Consultar `GET https://api.github.com/user` con el token emitido y extraer el ID remoto. |
| Guardar | Persistir la credencial en el almacén del SO y confirmar sus metadatos locales de forma recuperable. |

Las URLs de autorización serán constantes del backend. Los tokens, códigos de dispositivo y cuerpos de error sensibles permanecerán dentro de Rust. La cuenta aparecerá conectada después de verificarla y completar su almacenamiento. [Flujo de dispositivo de GitHub](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token)

| Estado o respuesta | Tratamiento previsto |
| --- | --- |
| `authorization_pending` | Esperar el intervalo autorizado antes de consultar otra vez. |
| `slow_down` | Aumentar el intervalo al menos cinco segundos y conservarlo durante la sesión. |
| Timeout de conexión | Aplicar espera creciente, respetar el plazo y ofrecer cancelar. |
| Denegación o código vencido | Cerrar la sesión y ofrecer iniciar una nueva autorización. |
| Cancelación | Detener solicitudes nuevas, cancelar la petición activa y descartar respuestas tardías. |
| Cierre de GitPilot | La sesión temporal termina. La próxima apertura puede iniciar otra. |

La máquina de estados admitirá una petición activa por sesión. El presupuesto de espera y los reintentos se comprobarán con reloj simulado antes de usar la red. [OAuth Device Authorization Grant](https://www.rfc-editor.org/rfc/rfc8628#section-3.5)

## Credenciales y recuperación

Los adaptadores previstos son Windows Credential Manager y Linux Secret Service. Una entrada por versión de credencial contendrá access token, refresh token y vencimientos. SQLite conservará el proveedor, Client ID, ID de usuario, login, referencia al almacén, revisión y estado de conexión. Los nombres visibles se tratarán como datos remotos no confiables.

La renovación será única por cuenta, también entre instancias de GitPilot. GitHub invalida los tokens anteriores cuando rota la credencial. Una respuesta ambigua tras renovar requiere recuperación o nueva autorización antes de volver a usar el refresh token anterior. [Rotación de credenciales de usuario](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens)

El journal de credenciales registrará primero una referencia pendiente. Tras escribir y comprobar la nueva entrada segura, una transacción actualizará la cuenta y marcará la referencia anterior para retirada. Al reiniciar, se reconciliarán las entradas pendientes. Una retirada fallida quedará visible y admitirá reintento. El adaptador deberá fallar de forma explícita si el almacén está bloqueado o indisponible.

La desconexión cancelará las operaciones de la cuenta, retirará sus secretos locales y limpiará su caché. Abrirá, a petición del usuario, la sección **Authorized GitHub Apps** en GitHub para revocar la autorización remota. La interfaz distinguirá la retirada local verificada de la acción remota pendiente. La API administrativa de revocación requiere autenticar a la aplicación con su secreto. [Revocación de autorizaciones](https://docs.github.com/en/rest/apps/oauth-applications#delete-an-app-authorization)

## Consultas y asociaciones

Las peticiones autenticadas se ejecutarán en Rust contra endpoints HTTPS permitidos de `api.github.com`. Se fijarán `Accept`, User-Agent y una versión soportada de la API al implementar el cliente. Las redirecciones y los enlaces de paginación requerirán validación del host y la ruta antes de reenviar una credencial.

| Recurso | Consulta |
| --- | --- |
| Cuenta | `GET /user` |
| Instalaciones | `GET /user/installations` |
| Repositorios de una instalación | `GET /user/installations/{installation_id}/repositories` |
| PR | `GET /repos/{owner}/{repo}/pulls` |
| Issues | `GET /repos/{owner}/{repo}/issues`, separando las entradas que contienen `pull_request`. |

La caché y sus claves incluirán la cuenta, la instalación, el recurso y la página. Cada cambio de cuenta invalidará las consultas anteriores. El cliente respetará las indicaciones de limitación del servicio, acotará tamaño y tiempo y distinguirá falta de permisos, recurso inaccesible, sesión vencida y fallo de red. [Uso de la API REST](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api)

Para asociar un remoto, el backend volverá a consultar Git y verificará que el repositorio remoto sea accesible para la cuenta elegida. Se guardará la asociación por IDs estables, junto con la huella del destino leído. Si Git cambia ese destino externamente, la asociación requerirá revisión.

## Entregas de la fase 6

| Entrega | Implementación | Criterio verificable |
| --- | --- | --- |
| 6.0 | Decisión, permisos, registro y contratos | ADR adoptado, fuentes revisadas y guía disponible. Completada en esta entrega. |
| 6.1 | Device Flow, cancelación, almacenes seguros y primera cuenta | Implementada. Autorización real en Windows y flujo simulado con Secret Service en Linux. [Evidencia](../testing/TESTING.md#evidencia-de-fase-6--primera-cuenta). |
| 6.2 | Varias cuentas, renovación y desconexión | Aislamiento por ID, rotación serializada, recuperación y retirada local comprobadas. Revocación remota recorrida por el usuario. |
| 6.3 | Instalaciones, organizaciones y repositorios | Acceso privado y selección de repositorios comprobados, con estados de permisos y SSO. |
| 6.4 | PR, issues y asociación con el inventario | Paginación, rate limits, aislamiento de caché y cambio externo de remoto comprobados. |

Cada entrega usará una rama breve desde `main` y commits convencionales por responsabilidad. La fase termina cuando las comprobaciones de escritorio, almacenes, permisos y recuperación de estas entregas pasan en ambas plataformas.

## Comportamiento de la entrega 6.1

La pantalla GitHub inicia la autorización mediante una acción del usuario, muestra el código y permite abrir la URL de verificación o cancelar. El backend conserva la sesión temporal y aplica los intervalos con un reloj monotónico. La cancelación interrumpe la petición activa y descarta sus respuestas tardías. Tras el canje se verifica `/user` y se guarda la credencial completa en el almacén del sistema.

La sesión persistida conserva ambos tokens y sus vencimientos para la futura renovación. La interfaz permite comprobar el acceso y volver a autorizar la misma cuenta. Al vencer el access token, esta entrega requiere otra autorización. La conexión de cuentas adicionales, la renovación automática y la desconexión se incorporarán en 6.2.

El journal guarda referencias `pending`, `active` y `retiring`. Las escrituras de credenciales se verifican antes de actualizar la cuenta en SQLite. Las entradas huérfanas se retiran al cargar o guardar la cuenta bajo un lock del sistema. En Linux la retirada comprueba también la ausencia de la entrada, incluida la recuperación de un journal creado antes de escribir el secreto. Una retirada pendiente permanece visible.

## Validación requerida

- Servidor simulado y reloj controlado para espera, `slow_down`, denegación, expiración, cancelación, respuestas excesivas y redirecciones.
- Almacenes temporales o credenciales de QA identificadas de forma única para guardado, reapertura, bloqueo, borrado, escrituras interrumpidas y reconciliación.
- Dos cuentas de prueba, un repositorio privado seleccionado y otro excluido, además de una instalación aprobada en una organización.
- Fallos durante la rotación y la desconexión, incluidos cambios concurrentes y expiración entre una consulta y su respuesta.
- Inspección de IPC, logs y SQLite para comprobar que contienen solo los metadatos previstos. La evidencia distinguirá fixtures y servicios reales.
