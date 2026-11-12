# Pruebas y validación

## Evidencia de aislamiento de perfiles

Validación del 12 de noviembre de 2026 en Windows, Git 2.55.0.windows.5:

- La integración usa un subproceso con HOME/USERPROFILE/XDG temporales, Git de sistema desactivado, SQLite y repositorios temporales. Comprueba sustitución de autor global y de un include local compartido, cambio de un perfil con claves a otro sin claves con el mismo autor, firma opcional y limpieza de la clave anterior, rechazo de archivos ausentes y de confirmaciones vencidas. OpenSSH `-G` comprueba los archivos y métodos configurados sin abrir conexiones. Un commit externo confirma autor y committer; un repositorio futuro hereda el perfil actualizado. La restauración recupera los bytes originales y conserva el include compartido.
- Se comprueban reaplicaciones, orden de restauración, estado de operaciones anteriores y recuperación de una publicación interrumpida. Las rutas de clave incluyen espacios, comillas y metacaracteres de shell y de OpenSSH.
- La revisión visual con IPC ficticio cubre 900 × 600 y reflujo a 390 px, selección/cancelación de claves, firma opcional, foco de la revisión y aplicación. En WebView2, una instancia con identificador de prueba separado abrió los diálogos nativos de archivos y carpetas y devolvió las rutas sintéticas correctas a los campos de solo lectura.
- Pasaron la compilación frontend, ESLint, TypeScript, cuatro pruebas Vitest, Clippy con avisos como errores y la suite Rust (40 aprobadas y cinco ignoradas por sus requisitos explícitos). Se compiló el ejecutable Tauri de depuración con `custom-protocol`.

Límites: la integración de escritura y los selectores nativos se verificaron en Windows; esta entrega no acredita Linux, acceso remoto ni firmas criptográficas reales. La asignación modifica repositorios ordinarios conocidos y publica reglas para futuros. Repositorios fuera del inventario con valores locales propios, worktrees vinculados y variables/opciones externas requieren su diagnóstico. Una asignación de varios archivos es recuperable por archivo; si un paso posterior falla, los anteriores permanecen visibles en Actividad. HTTPS conserva sus credenciales externas; los alias y saltos de archivos SSH quedan fuera del comando aislado del perfil.

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
| `release.yml` | Ejecución manual desde `main` con CI verificada para el mismo commit. Genera NSIS, Debian, metadatos y SHA-256 con permisos de lectura. La firma, validación de los archivos finales y publicación se realizan según la [guía de distribución](../RELEASING.md). |

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

## Evidencia de fase 7 · Escaneo del inventario

- En Windows 11, una raíz de trabajo con 11 repositorios directos tenía guardado un escaneo parcial con uno. El nuevo escaneo desde WebView2 verificó los 11 directos y un repositorio anidado en 5,9 segundos. El aviso informó del límite de entradas. Se compararon los archivos `.git/config` de los 11 repositorios directos y se verificaron la ventana de 900 × 600 y el reflujo emulado a 390 px.
- En Debian 12, un fixture con 12 200 archivos y cuatro repositorios, incluido uno anidado, agotó el límite de entradas. WebKitGTK conservó los cuatro resultados y mostró la causa del escaneo parcial. El escaneo y la persistencia tardaron aproximadamente 270 ms en ese entorno.
- Pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy y compilación Tauri en Windows y Linux. La suite Rust pasó 32 pruebas en Windows y 33 en Linux. El nuevo fixture comprueba que, ante un subárbol grande, el escaneo conserva los repositorios hermanos y verifica los candidatos anidados que quedaron en cola al alcanzar el límite.

## Evidencia de fase 7 · Preferencias y control de escaneo

- En Windows 11 y Debian 12 pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy y compilación Tauri. La suite Rust pasó 35 pruebas en Windows y 36 en Linux. Los fixtures verifican valores predeterminados, persistencia, rechazo de valores inválidos y escrituras obsoletas, exclusividad del trabajo, cancelación por ID y publicación parcial.
- En WebView2 se rechazó un límite inválido, se guardaron preferencias y se leyeron tras reabrir GitPilot. Un escaneo posterior respetó la profundidad cero. Se comprobó la carga de valores predeterminados en el borrador, el reflujo emulado a 390 px y el retorno de foco. Tras cancelar un escaneo activo, el inventario conservó cuatro repositorios del fixture, incluido el que estaba registrado antes.
- En WebKitGTK se editaron y reabrieron las preferencias. Durante un escaneo con 261 repositorios temporales se observaron los conteos, se canceló por teclado y se conservaron 248 repositorios, incluido el resultado anterior. La ventana nativa de ambos ensayos fue de 900 × 600.

## Evidencia de fase 7 · Portabilidad, idioma y actividad

- En Windows 11 y Debian 12 pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy y la compilación Tauri. La suite Rust pasó 39 pruebas en Windows y 40 en Linux. Los fixtures cubren versión desconocida, tamaño, campos y referencias inválidos, rutas con retroceso, preservación de reglas y archivos existentes, revisión obsoleta y reversión de la transacción si falla el registro.
- En WebView2 se rechazó un archivo de versión desconocida, se revisó y canceló una importación y se comprobó el retorno de foco. Después de confirmar, se filtró Actividad, se reabrió GitPilot en inglés y se guardó español desde Configuración. Pasaron la ventana fija de 900 × 600 y el reflujo emulado a 390 px.
- En WebKitGTK se eligió un JSON desde el selector de archivos, se revisó una ruta ausente, se confirmó y se exportó un archivo nuevo con el selector nativo. Se verificaron el esquema y el contenido del JSON, las dos entradas de Actividad, la reapertura en inglés y la ventana de 900 × 600.
- Las dos pruebas de interfaz usaron bases locales separadas y archivos Git temporales. La configuración Git coincidió byte a byte antes y después. El alcance inicial del inglés cubre navegación, encabezados, Inicio, Configuración y el listado de Actividad. Los formularios y diagnósticos detallados están en español.

## Evidencia de fase 7 · Validación de la beta

### Rendimiento

Desde `apps/desktop/src-tauri`, ejecuta el fixture con Git disponible en PATH. El proceso de prueba usa un directorio temporal y aísla HOME, SQLite y la configuración Git.

```sh
cargo test --locked --features custom-protocol beta_probe::scan_and_diagnosis_budget -- --ignored --nocapture
```

Con las preferencias predeterminadas, el fixture registró 100 repositorios en 10 grupos y recorrió 5 020 entradas, incluidas 4 800 de archivos comunes. El escaneo incluye el guardado en SQLite. El diagnóstico p95 se calcula sobre 30 repositorios con una identidad Git distinta de la esperada. La cancelación se solicitó al recibir el primer aviso de recorrido y conservó los 100 repositorios anteriores.

| Métrica | Presupuesto del fixture | Windows 11 x64, Git 2.55.0.windows.5 | Debian 12 amd64 en Docker, Git 2.39.5 |
| --- | ---: | ---: | ---: |
| Escaneo y guardado | ≤ 15 s | 6 073 ms | 516 ms |
| Publicación en SQLite | Incluida en el escaneo | 14 ms | 33 ms |
| Diagnóstico p95 | ≤ 1 s | 139 ms | 8 ms |
| Cancelación y guardado | ≤ 1 s | 8 ms | 12 ms |
| Memoria residente pico del proceso del fixture | Referencia inicial | 13,2 MiB | 13,5 MiB |

Los tiempos proceden del perfil de pruebas local. La memoria se observó mediante el conjunto de trabajo en Windows y RSS en Debian. El fixture mide la lógica Rust, SQLite y Git. La memoria de los webviews y el rendimiento en unidades de red requieren una medición aparte.

### Accesibilidad y paquetes

- En WebView2, axe-core 4.14.0 ejecutó reglas WCAG A y AA en 12 estados de las pantallas principales. Obtuvo cero infracciones automáticas. Los resultados de contraste pendientes aparecieron en contenido con desplazamiento y se revisaron con los colores del tema. Los bordes de controles pasaron de un contraste mínimo de 2,47:1 a 3,13:1 sobre las superficies probadas. Se verificaron orden de tabulación, retorno de foco, revisión de importación con acciones visibles, movimiento reducido y reflujo a 390 px. El foco se corrigió para llegar al título al cambiar de sección. [Teclado y foco visible en WCAG](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html)
- En WebKitGTK se importó y guardó una identidad con el teclado, se registró un directorio, se escaneó un repositorio y se abrió su diagnóstico. Tras reinstalar, la identidad y el repositorio seguían disponibles. Se comprobó que el foco de teclado llegaba al título de Directorios y que el área de contenido seguía en 900 × 600. Las pruebas cubrieron teclado y foco visible. Queda pendiente una revisión con lectores de pantalla.
- Se instalaron, reinstalaron y desinstalaron el NSIS x64 con un identificador de QA y el paquete Debian amd64 en un contenedor. Se verificó el PNG instalado en Debian y se compararon los archivos de configuración Git de prueba antes y después. Se generaron candidatos locales de v0.1.0 con sumas SHA-256. La firma de distribución y la publicación corresponden a la fase 8.
- En Windows y Debian pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy y compilación Tauri. La suite Rust pasó 39 pruebas en Windows y 40 en Debian. El fixture de rendimiento se ejecutó aparte en ambas plataformas.

## Evidencia de fase 8 · Candidato v1.0.0

- En Windows y Debian pasaron Prettier, ESLint, TypeScript, cuatro pruebas Vitest, formato Rust, Clippy y empaquetado Tauri. Pasaron 39 pruebas Rust en Windows y 40 en Debian.
- En WebView2 se verificó el PNG de GitPilot en las seis secciones, la ventana fija de 900 × 600 y el reflujo a 390 px. En WebKitGTK se verificó el logo al reabrir Identidades en la ventana fija.
- Se actualizó el NSIS de 0.1.0 a 1.0.0 con un identificador aislado de QA. Se verificaron los perfiles, los repositorios y los archivos de licencia y atribución instalados. En Debian se actualizó el paquete de 0.1.0 a 1.0.0, se comprobó SQLite y se compararon los archivos Git global y local del fixture byte a byte.
- `actionlint` validó el workflow del candidato. Se comprobó la consulta de CI para un SHA exacto de `main` y el rechazo de versiones desalineadas en los manifiestos. Los candidatos locales tienen SHA-256. La ejecución de GitHub Actions corresponde al commit del candidato una vez integrado en `main`. [Proceso de distribución](../RELEASING.md)


## Cierre 1.1.0: rutas temporales de CI

La primera ejecución de Desktop CI del cierre detectó una confirmación ausente en el fixture de asignación. El fixture ahora canonicaliza su carpeta temporal antes de pasar HOME, USERPROFILE y XDG_CONFIG_HOME al proceso hijo, evitando que un alias Windows 8.3 se compare con su ruta larga al comprobar destinos seguros. Conserva la validación estricta de rutas de producción y muestra el plan completo del fixture si falta confirmación.

La prueba pasó localmente tanto con TEMP habitual como con TEMP/TMP apuntando a un alias 8.3 real obtenido mediante GetShortPathNameW. Las bases, repositorios y configuraciones usados fueron temporales. La validación remota del commit corregido debe pasar antes de construir el candidato.


El diagnóstico posterior detectó un cambio real del descriptor de seguridad al reemplazar archivos en Windows. La regresión `replacement_preserves_explicit_windows_security_group` asigna un grupo explícito a un archivo temporal: falló antes de la corrección y pasó tras copiar y verificar propietario, grupo y ACL sobre el archivo de staging antes del reemplazo. La comprobación estricta de permisos permanece activa. Clippy y las 42 pruebas Rust habilitadas pasaron localmente; las cinco pruebas especiales ya tenían evidencia local de ejecución separada. CI del nuevo commit debe confirmar el resultado en el runner.

### Herencia ACL de Windows Server durante el cierre 1.1.0

CI detectó que ReplaceFileW duplicaba las ACE heredadas en ACL sin el indicador AI. El motor publica el staging verificado mediante MoveFileExW en el mismo directorio. Las pruebas `replacement_preserves_explicit_windows_security_group`, `replacement_preserves_legacy_inherited_windows_acl` y `refuses_windows_configuration_with_alternate_streams` verifican metadatos y rechazo seguro sin eliminar streams. El rechazo se extiende a archivos comprimidos, cifrados y sparse. La huella del journal conserva su formato anterior.

### Interacción Windows y zoom en el cierre 1.1.0

Instancia con identificador `dev.gitpilot.releaseqa.closure20261112`, base sintética y Git global redirigido a una fixture. Verificados menú vertical, tres reglas sin duplicar el repositorio bajo raíces solapadas, rutas largas completas, carpetas homónimas, desplazamiento con Ctrl+Fin, foco visible con Tab y navegación a Directorios con Enter. No se escribieron configuraciones Git desde la interfaz.

Se detectaron atajos de zoom desactivados por defecto en WebView2. `tauri.windows.conf.json` activa `zoomHotkeysEnabled` conservando las dimensiones y propiedades de la ventana. En la instancia aislada se comprobaron los cinco incrementos nativos de Ctrl+numpad+, de 100% a 200%, reflujo a una columna y acceso a los controles finales con Ctrl+Fin; Ctrl+0 restableció el tamaño. No se modifican capacidades IPC ni configuración de Linux. La revisión no acredita todos los estados de error ni una auditoría WCAG completa.
