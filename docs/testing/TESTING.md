# Pruebas y validación

## Capas de prueba

| Nivel | Herramienta y objetivo |
| --- | --- |
| Dominio Rust | `cargo test` para identidad, reglas, precedencia, conflictos, estados de operaciones y validaciones. |
| Integración Rust y Git | SQLite temporal, migraciones y repositorios Git reales. Verificar configuración y crear commits externos a la aplicación. Inyectar fallos en cada etapa de escritura. |
| Componentes | Vitest para formularios, validación, estados, mensajes y accesibilidad de componentes. |
| Flujos UI | Playwright para flujos del frontend en navegador y contratos IPC con fixtures. Complementar con pruebas de escritorio reales del IPC, webview y empaquetado. |
| Escritorio e instalador | Smoke tests en cada SO. Evaluar WebdriverIO con soporte Tauri para automatizar escenarios nativos, limitando el driver a builds de prueba. Validar instalación y flujos críticos en los artefactos de distribución. [Pruebas WebDriver en Tauri](https://v2.tauri.app/develop/tests/webdriver/) |
| Seguridad y rendimiento | Revisar permisos, dependencias, binarios, redacción y entradas hostiles. Medir duración, memoria y cancelación de escaneos sobre fixtures reproducibles. Fijar presupuestos a partir de la primera implementación de escaneo en fase 3. |

Los fixtures aíslan HOME y XDG del proceso de prueba, redirigen Git global y desactivan Git de sistema, limpian variables de identidad o rutas heredadas y emplean DB, raíces y repositorios temporales. Ninguna prueba escribe la configuración habitual del desarrollador. Usar datos sintéticos y evitar credenciales reales en PRs.

La [prueba de viabilidad de fase 0](PHASE_0.md) sirve de referencia para ampliar los fixtures de integración cuando exista el motor.

## Matriz crítica

| Área | Casos mínimos |
| --- | --- |
| Identidades y datos | CRUD, duplicado, correo inválido, dos perfiles con el mismo correo, eliminación en uso, predeterminado optativo, reinicio y migración desde una base de la versión anterior. |
| Escaneo | Directorio inexistente, sin permisos, excluido, rutas Unicode o con espacios, límite de recursos, cancelación, enlaces y junctions, anidación, submódulo, bare y worktree en otra ruta. |
| Resolución | Sin regla, predeterminado, reglas solapadas, empate de normalización, override, nombre y correo de orígenes distintos, conflicto conservado, configuración global doble, includes y entorno externo. |
| Configuración y commits | Rutas con glob, variantes de mayúsculas, repositorio nuevo, colisión local, permisos y archivos ausentes, reintentos y rollback. Verificar desde terminal con GitPilot cerrado tras aplicar y después de restaurar. Ejecutar M1–M6 del [PRD](../product/PRD.md). |
| Recuperación | Backup fallido, bloqueo de archivo, dos instancias, edición entre vista previa y aplicar, fallo tras cada escritura, cierre abrupto, verificación fallida y cambios ajenos después de aplicar. |
| Seguridad | IPC y rutas maliciosos, metacaracteres, binario inesperado, configuración con hooks/helpers hostiles, salida excesiva, timeout, filtración en logs, DB, reportes y respaldos. |
| Autenticación posterior | Clave ausente o inaccesible, host desconocido, cuenta sin acceso, token expirado o revocado, permisos y SSO, repositorio privado, cancelación y keychain indisponible. |

## GitHub Actions y publicación

| Workflow previsto | Activación y criterio |
| --- | --- |
| `ci.yml` | En PR a `main` y push al tronco. Revisar rutas ignoradas que se hayan rastreado, ejecutar Prettier, ESLint, typecheck, Vitest, formato Rust, Clippy, `cargo test` y build Tauri. Matriz Windows x86_64 y Linux x86_64. Incorporar flujos UI a medida que se implementen. |
| `security.yml` | Dependabot, auditoría de dependencias Rust/JS y análisis estático según soporte. Evaluar CodeQL y `cargo deny`. Bloquear hallazgos críticos según política. |
| `release.yml` | Tags versionados de `main` con CI verificada. Construir, firmar donde proceda, generar checksums, comprobar instalación y publicar mediante permisos acotados y secretos del entorno de release. |

La definición de terminado exige criterios de aceptación satisfechos, validaciones ejecutadas y registradas, cobertura de todos los casos críticos aplicables y documentación actualizada. Toda escritura Git requiere pruebas de integración. Los casos de cada plataforma se registran con versión de Git, sistema, resultados y límites. La publicación exige resultados favorables en ambas plataformas objetivo.

## Evidencia de fase 1

| Plataforma | Validación ejecutada |
| --- | --- |
| Windows 11 x86_64 | Compilación Tauri y arranque con WebView2. El texto de la ventana confirmó la respuesta IPC y se comprobó la migración aplicada en SQLite. |
| Debian Bookworm x86_64 sobre WSL2 | Compilación Tauri y arranque con WebKitGTK en una pantalla virtual. Se comprobó el estado visible de almacenamiento y la migración en un directorio temporal tanto con el binario compilado como con `npm run tauri -- dev`. |

Prettier, ESLint, typecheck, cuatro pruebas de Vitest, formato de Rust, Clippy y dos pruebas de almacenamiento Rust pasaron en ambos entornos. La [CI](../../.github/workflows/ci.yml) reproduce las comprobaciones de compilación y pruebas cuando los cambios se publiquen en GitHub.

## Evidencia de fase 2

- En Windows 11 x86_64 y Debian Bookworm x86_64 sobre WSL2 pasaron la compilación Tauri, los checks de formato y tipos, ESLint, Clippy, cuatro pruebas Vitest y siete pruebas Rust. Los casos Rust cubren validación, persistencia, duplicados con el mismo correo, revisiones obsoletas, predeterminado, protección de referencias y lectura global de Git.
- En WebView2 se recorrieron creación, rechazo de un correo inválido, edición, duplicación, confirmación de eliminación, selección predeterminada, cancelación y confirmación de importación y reapertura. Se comprobó el diseño compacto a 390 px en modo oscuro.
- En WebKitGTK se abrió la vista previa de Git, se guardó mediante teclado y se verificó el perfil en SQLite y tras reabrir. Los archivos de Git utilizados en las pruebas de interfaz fueron temporales y conservaron su contenido.

## Evidencia del rediseño

- En Windows se comprobó el área de 900 × 600 píxeles lógicos y el estilo nativo de ventana fija. En WebView2 se recorrieron creación, validación, edición, perfil predeterminado, duplicación, eliminación, importación y reapertura con datos aislados.
- En Linux se comprobó la geometría de 900 × 600 con Openbox. La ventana conservó sus dimensiones al solicitar un tamaño mayor. En WebKitGTK se revisó y guardó la importación mediante teclado, con un archivo de Git temporal.
- El inspector visual comprobó el reflujo a 768 y 390 px. La presentación de escritorio se revisó en capturas de la ventana nativa.

## Evidencia de fase 3

- En Windows 11 x86_64 (Git 2.55.0.windows.5) y Debian Bookworm x86_64 (Git 2.39.5) pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato de Rust, Clippy con avisos como errores, diez pruebas Rust y la compilación Tauri. Los fixtures cubrieron repositorios anidados, bare y worktrees, exclusiones, origen por campo, raíces solapadas, precedencia, protección de perfiles asignados y conservación del inventario al escanear parcialmente.
- En WebView2 se verificó la ventana fija de 900 × 600, el registro de una ruta con espacios, el escaneo y reescaneo, la asignación al directorio y al repositorio, la discrepancia con orígenes local y global, la exclusión y la persistencia al reabrir. Se compararon los archivos de configuración Git antes y después.
- En WebKitGTK, sobre una pantalla virtual, se registró el directorio desde la interfaz, se verificaron dos repositorios anidados en SQLite y se abrió el diagnóstico con la identidad global de un archivo temporal. Se verificó la geometría de 900 × 600 y se comparó el archivo de configuración del repositorio.
- El recorrido se detiene a los 12 segundos o al alcanzar los límites de entradas (12 000) o repositorios (200). Se omiten enlaces, junctions y ramas a más de 24 niveles de profundidad. La interfaz muestra los resultados parciales y admite un nuevo escaneo bajo demanda. Las exclusiones delimitan el descubrimiento. La cancelación manual y la validación de instaladores se reservan para entregas posteriores.

## Evidencia de fase 4 · Vista previa

- En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian Bookworm x86_64 con Git 2.39.5 pasaron la compilación Tauri, formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy y doce pruebas Rust. Los casos nuevos verifican selección de HOME y XDG, inclusiones relativas, rutas con espacios y corchetes, precedencia entre carpeta, predeterminado y excepción, y conservación del correo local.
- En WebView2 se registró un directorio temporal, se asignó un perfil y se abrió la vista previa. Se comprobó el destino global, el conflicto, el origen de los valores actual y simulado, el retorno de foco al inventario y el área fija de 900 × 600. En WebKitGTK se comprobó el escaneo y la vista previa con un perfil predeterminado y un correo local contradictorio.
- Los archivos Git globales y locales de los fixtures conservaron sus bytes durante la simulación.

## Evidencia de fase 4 · Aplicación y recuperación

- En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian Bookworm x86_64 con Git 2.39.5 pasaron compilación, formato, lint, TypeScript, cuatro pruebas Vitest y catorce pruebas Rust. Los casos de recuperación recrean un journal interrumpido y verifican tanto la compensación como la preservación de una edición externa. También se prueba la contención con un lock ajeno.
- En WebView2 se comprobó el rechazo de una vista previa vencida, la aplicación de una regla de carpeta y una excepción, la confirmación de restauración y el bloqueo tras una edición externa. Con GitPilot cerrado se crearon commits desde Git CLI y se verificó el nombre y correo de autor y committer. Un repositorio creado después de aplicar heredó la regla. La restauración recuperó los bytes anteriores.
- En WebKitGTK se comprobó la aplicación y la restauración con un almacén Secret Service temporal. SQLite conservó el respaldo cifrado. Git CLI confirmó autor y committer en un commit temporal y la herencia en un repositorio nuevo. Se compararon los bytes del archivo global restaurado y del archivo local.
- La validación de cierre abrupto usa un estado de interrupción reproducido en fixtures. Los cortes de energía físicos quedan pendientes.

## Evidencia de cierre de fase 4

En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian 12 amd64 con Git 2.39.5 pasaron formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy con avisos como errores y quince pruebas Rust. Se generaron los candidatos de release NSIS y Debian y se calculó su SHA-256.

| Caso | Validación |
| --- | --- |
| M1 | Se instalaron y desinstalaron el NSIS x64 y el paquete Debian amd64. En Windows se recorrió la aplicación instalada con un identificador aislado de QA. En Debian se abrió el paquete de release instalado. Inicio detectó Git y se comprobó la persistencia de tres perfiles en el fixture de Windows. |
| M2 | Se registraron dos raíces, se detectó un repositorio anidado, se excluyó una subcarpeta y se comprobó el estado sin asignación antes de vincular los perfiles. |
| M3 | Se asignó un tercer perfil como excepción. La vista previa y confirmación mostraron la prioridad y el destino global. |
| M4 | Con GitPilot cerrado, Git CLI creó commits de carpeta y excepción. Un repositorio nuevo heredó la regla. Una instancia portátil de VS Code Insiders 1.146.0 creó un commit mediante la API de su extensión Git. Se comprobó autor y committer desde Git. El VS Code estable instalado tenía una actualización en curso. |
| M5 | La decisión de conservar quedó en Actividad y perdió vigencia al cambiar el correo local. Windows y Debian aplicaron una corrección local explícita y verificaron la identidad. Una prueba Rust rechaza el origen local en un archivo incluido ajeno al repositorio. |
| M6 | Se restauraron los archivos global y local desde Actividad y se compararon los bytes. Las pruebas Rust cubren la interrupción simulada y la protección de una edición externa. |

Los artefactos de esta entrega son candidatos locales. La firma de los instaladores y la publicación formal quedan pendientes.

## Evidencia de fase 5 · Inspector local

- En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian 12 amd64 con Git 2.39.5 pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy con avisos como errores, diecisiete pruebas Rust y compilación Tauri.
- Los fixtures de Git comprobaron la rama, el índice y los destinos de lectura y envío, incluida la reescritura de URL. Los comandos configurados para filtros, monitores, SSH, credenciales y firma quedaron sin ejecutar. Los valores señuelo permanecieron fuera de la respuesta y se compararon los bytes del índice y la configuración.
- En WebView2 y WebKitGTK se abrió el Inspector en la ventana de 900 × 600. Se revisaron el host sin credenciales, un alias SSH simple, la presencia de claves, los helpers HTTPS y la configuración de firma. En Windows se verificó el retorno de foco. En Debian se comprobó la presencia del archivo de firma SSH a partir de sus metadatos.

## Evidencia de fase 5 · Comprobaciones y pre-commit

- En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian 12 amd64 con Git 2.39.5 pasaron formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy con avisos como errores y compilación Tauri. Pasaron diecinueve pruebas Rust en Windows y veinte en Debian, donde el caso adicional firma y comprueba un desafío con un agente SSH temporal.
- En WebView2 se confirmó la lectura HTTPS anónima de un repositorio público de GitHub. Se comprobaron las modificaciones rastreadas sin preparar y la conservación de los bytes del índice y la configuración. Los fixtures verifican que el diagnóstico omite este recuento ante filtros configurados.
- En WebKitGTK se confirmó la lectura SSH de un repositorio servido por un OpenSSH temporal y la firma SSH mediante un agente temporal. El servidor y la resolución del host quedaron aislados dentro del contenedor de pruebas, con su clave de host fijada. La interfaz mostró ambos resultados favorables en la ventana fija de 900 × 600.
- En Windows, la instalación confirmada del hook permitió un commit con identidad discrepante en modo aviso y lo rechazó en modo bloqueo. El modo bloqueo permitió la identidad coincidente. La retirada restauró la ausencia del archivo, mientras que una edición externa dejó la operación pendiente de recuperación y conservó el contenido ajeno. Las pruebas Rust crean commits reales en repositorios temporales y cubren identidades con apóstrofes.
- Esta entrega limita las pruebas remotas a destinos directos compatibles de GitHub, GitLab y Bitbucket. La prueba HTTPS es anónima y la prueba de firma usa SSH con agente. La autenticación HTTPS mediante helpers, la firma OpenPGP y las configuraciones SSH complejas siguen pendientes. El hook conserva la identidad revisada al instalarlo y requiere reinstalación para actualizarla.

## Evidencia de fase 5 · HTTPS autenticado, ruta SSH y OpenPGP

- En Windows 11 x86_64 con Git 2.55.0.windows.5 y Debian 12 amd64 con Git 2.39.5 pasaron formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy con avisos como errores y la compilación Tauri. Pasaron 22 pruebas Rust en Windows y 23 en Debian.
- En WebView2 se firmó y verificó un desafío con el GnuPG de Git for Windows y un llavero temporal. Se revisó una ruta SSH con alias, inclusión, puerto alternativo y salto. El cambio del puerto invalidó la revisión y una directiva `Match exec` requirió revisión manual. La prueba HTTPS detuvo un helper personalizado, conservó la configuración del repositorio y devolvió el foco al botón de origen.
- En WebKitGTK se comprobó la lectura de un repositorio privado de prueba con TLS validado, credenciales temporales de `credential-cache` y una regla por URL que reinicia la lista de helpers. El servidor comprobó la credencial en la solicitud. También respondió un repositorio SSH a través de un salto con puertos y claves de host fijados y se firmó un desafío OpenPGP con un llavero temporal. Se compararon los bytes de `.git/config`, los archivos SSH y `known_hosts`.
- La prueba HTTPS positiva usó `credential-cache` y un servidor aislado. Las sesiones reales de GCM, Windows Credential Manager y Secret Service quedan fuera de esta validación. El alcance de directivas SSH y referencias OpenPGP se documenta en la [especificación](../security/AUTHENTICATION.md#comprobaciones-explícitas).

## Evidencia de fase 6 · Decisión y contratos

La entrega 6.0 adopta GitHub App con Device Flow y permisos de lectura. Se contrastaron con la documentación oficial el canje público, la renovación de credenciales originadas por Device Flow, la revocación administrativa y los permisos de los endpoints previstos. El [ADR-004](../decisions/ADR-004-autenticacion.md) y la [especificación](../architecture/GITHUB.md) enlazan esas fuentes.

Esta entrega modifica documentación. Se comprobaron los enlaces internos y el diff. El ensayo de autorización real requiere el registro de desarrollo y su Client ID público. Los casos de red, almacenes, varias cuentas y organizaciones quedan definidos para las siguientes entregas de la fase.

## Evidencia de fase 6 · Primera cuenta

- En Windows 11 x86_64 y Debian 12 amd64 pasaron formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy y compilación Tauri. La suite normal pasó 25 pruebas Rust en Windows y 26 en Linux. La prueba adicional del almacén del SO, excluida por defecto para mantener la CI aislada, se ejecutó expresamente y pasó en ambas plataformas.
- Las pruebas HTTP locales cubren la espera mínima, `slow_down`, rechazo de autorización, vencimiento, cancelación de una petición activa, respuesta tardía, URL de verificación incompatible y tamaño de respuesta. La prueba del almacén verifica la cuenta, reabre SQLite, consulta la credencial persistida y reconcilia referencias pendientes con y sin un secreto escrito.
- En Windows se completó Device Flow contra GitHub con la aplicación registrada y autorización manual del responsable. WebView2 mostró la cuenta verificada. Tras cerrar y reabrir se recuperó la sesión de Windows Credential Manager y `/user` respondió de nuevo. Se comprobaron SQLite y WAL en busca de prefijos de tokens.
- En Linux se utilizó un Secret Service de gnome-keyring con HOME y XDG temporales. La verificación y reapertura de la cuenta usaron un servidor HTTP simulado y el almacén real. El ensayo OAuth real se realizó en Windows. La renovación, varias cuentas, desconexión y consulta del repositorio privado corresponden a las entregas siguientes.
- WebView2 comprobó la cancelación de una reautorización conservando la cuenta guardada, el retorno de foco y el reflujo a 390 px. WebKitGTK recorrió la solicitud y cancelación de un código real en la ventana de 900 × 600.

## Evidencia de fase 6 · Ciclo de vida de cuentas

- En Windows 11 y Debian 12 pasaron formato, ESLint, TypeScript, cuatro pruebas de Vitest, Clippy y la compilación Tauri. La suite Rust pasó 26 pruebas en Windows y 27 en Linux. Ambas plataformas pasaron también las dos pruebas ejecutadas expresamente con Windows Credential Manager y Secret Service.
- Con un servidor HTTP local y cuentas temporales se comprobó la rotación de una credencial vencida, la recuperación de la credencial nueva tras reabrir SQLite, la ausencia de un segundo canje cuando la respuesta es ambigua y la retirada de una cuenta conservando la otra. La suite ordinaria comprueba que la migración conserva la referencia anterior y sus metadatos.
- En WebView2 se migró una cuenta real guardada, se cargó su foto y se volvió a verificar la sesión con GitHub. Al bloquear las imágenes se mostró la inicial. Pasaron la cancelación de la revisión de desconexión, la devolución de foco y el reflujo a 390 px en emulación. La ventana nativa conservó 900 × 600.
- En WebKitGTK se seleccionaron dos cuentas de prueba y se comprobaron sus fotos y el detalle a 900 × 600. La retirada local por la interfaz dejó un recordatorio, entregó la URL de autorizaciones al navegador de prueba y conservó la otra cuenta al marcar la revisión.
- La comprobación con GitHub usó una cuenta real con una sesión vigente. La rotación y la retirada de secretos se verificaron con credenciales temporales. La revocación de una autorización real queda para una cuenta de prueba.

## Evidencia de fase 6 · Repositorios autorizados

- En Windows 11 y Debian 12 pasaron formato, ESLint, TypeScript, cuatro pruebas de Vitest, Clippy y compilación Tauri. La suite Rust pasó 28 pruebas en Windows y 29 en Linux. Las tres pruebas explícitas con el almacén del sistema pasaron en ambos entornos.
- En Windows se consultó GitHub con la sesión guardada. La instalación personal de `@JhosepAC` tenía alcance de repositorios seleccionados y devolvió únicamente `JhosepAC/test-gitpilot`, identificado como privado. Se comprobó el regreso a la cuenta con devolución de foco, la ventana fija de 900 × 600 y el reflujo emulado a 390 px.
- En WebKitGTK, con Secret Service aislado y un servidor local, se recorrieron dos cuentas, una instalación de organización, dos páginas y un repositorio privado. Se cambió de cuenta mientras la consulta anterior seguía en curso y se comprobó el estado vacío de la cuenta nueva.
- Los fixtures verifican qué credencial usa cada cuenta, la paginación con ruta reconstruida, la ausencia de URLs remotas en IPC, los estados de permisos y SSO, el rechazo de una sesión y el plazo de reintento sin solicitudes durante el límite. La comprobación con GitHub usó una instalación personal. La instalación de organización y SSO se probaron con respuestas simuladas.

## Evidencia de fase 6 · Actividad y vínculos

- En Windows 11 y Debian 12 pasaron formato, ESLint, TypeScript, cuatro pruebas Vitest, Clippy y compilación Tauri. La suite Rust pasó 31 pruebas en Windows y 32 en Linux. Las cuatro pruebas explícitas con Windows Credential Manager y Secret Service pasaron en ambos sistemas.
- En WebView2 se consultaron el PR n.º 2 y el issue n.º 1 reales del repositorio privado `JhosepAC/test-gitpilot`. El listado de issues excluyó el PR. Se verificaron la ventana nativa de 900 × 600 y el reflujo emulado a 390 px.
- Con un repositorio local temporal, el cambio del remoto entre vista previa y confirmación bloqueó el guardado. Se guardó un vínculo válido, se detectó un cambio de destino posterior y se revisó el vínculo. El Inspector mostró el estado actualizado y retiró la asociación. La configuración de Git coincidió byte a byte antes y después del guardado inicial y de la retirada.
- En WebKitGTK se recorrieron las vistas de pull requests e issues con una instalación de organización simulada y Secret Service aislado. Los fixtures verifican también el rechazo de un repositorio que ya no figura en la instalación, la paginación cuando una página de issues queda vacía y la detención ante un límite agotado entre consultas. La comprobación contra GitHub usó una instalación personal.
