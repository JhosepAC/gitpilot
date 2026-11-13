# Plan de cierre y publicación de GitPilot 1.1.0

Fecha de inicio: 2026-11-12. Estado: **publicación y controles de distribución completados; límites de cobertura T08–T09 registrados**.

Objetivo: resolver las alertas pendientes, validar los cambios y distribuir instaladores de Windows x64 y Debian 12 amd64 desde un mismo commit verificado de `main`.

Este documento organiza el trabajo; no acredita que las tareas pendientes se hayan ejecutado. Los hallazgos iniciales corresponden a la [revisión de seguridad](../security/REVIEW_1_1_0.md). Revalidar avisos y versiones antes de elegir una actualización: pueden haber cambiado desde esa revisión.

## Cómo usar este plan

- Marcar una tarea solo cuando cumpla su criterio de aceptación y tenga evidencia registrada.
- Trabajar en ramas breves desde `main`, con un objetivo por entrega y Conventional Commits en inglés. Integrar las correcciones revisadas antes de construir el candidato definitivo.
- Usar bases, repositorios, claves y almacenes de credenciales de prueba aislados. No probar migraciones, instalación o recuperación sobre datos personales.
- Registrar por tarea: responsable, fecha, SHA, sistema, comandos, resultado y enlace a evidencia sin secretos. Si se bloquea, anotar causa y siguiente acción.
- Cualquier cambio de código, dependencias o configuración de compilación después de congelar el candidato exige un nuevo SHA, CI, artefactos, sumas y validaciones afectadas.
- Las alertas conocidas deben corregirse o tener una decisión explícita del responsable basada en alcance demostrado. Ocultar una alerta o excluir un destino no equivale a corregirla. La excepción no puede presentarse como «cero vulnerabilidades».

## Ajuste de alcance autorizado el 2026-11-12

El responsable autorizó commits, integración, preparación y publicación si los controles restantes son satisfactorios. Solicitó omitir la revisión y las pruebas adicionales de Linux. Se conservan los resultados Linux ya ejecutados, pero las comprobaciones omitidas no se marcan como aprobadas ni se supone evidencia inexistente. Se mantiene la compilación del paquete Debian y la auditoría del lockfile completo. Esta decisión sustituye la exigencia de nuevas validaciones Linux de T07–T09 y T12.

## Punto de partida

| Elemento | Evidencia disponible | Trabajo pendiente |
| --- | --- | --- |
| Código | Cambios integrados en `main`, SHA inicial `31eb0b3` | Corregir bloqueos y fijar un SHA final nuevo |
| npm | minimatch fijado en 11.0.1; auditoría sin alertas en la revisión inicial | Revalidar desde instalación limpia |
| Rust | 40 pruebas aprobadas, 5 omitidas; formato y Clippy correctos en Windows | Resolver alertas; repetir tras cambios y validar Linux |
| Seguridad | Dos vulnerabilidades y dos advertencias en Cargo | T02–T05 |
| Interfaz | Compila; revisión visual incompleta | T09 |
| Distribución | NSIS local, sumas y procedencia de candidato | CI remoto, instalación, actualización, firma y publicación |

## Orden y dependencias

`T01 → T02/T03/T04/T05 → T06 → T07/T08/T09 → T10 → T11 → T12 → T13 → T14 → T15`

T02–T05 pueden investigarse en paralelo, coordinando cualquier cambio del mismo lockfile. T09 puede adelantarse, pero debe revisarse de nuevo si las correcciones cambian la interfaz. No pasar a publicación mientras queden bloqueos sin resolver.

## Tareas de seguridad

### T01 · Reproducir las alertas y determinar su alcance

- [x] Completar T01.

**Acciones:**

1. Registrar versiones de Node, npm, Rust, Cargo, Git, sistema y herramientas de auditoría.
2. Ejecutar `npm ci` y `npm audit` en `apps/desktop`.
3. Ejecutar `cargo audit` en `apps/desktop/src-tauri`, registrando la revisión de la base RustSec.
4. Inspeccionar `cargo tree --locked --target all -i <paquete>` y los árboles de los destinos Windows y Linux para `libsqlite3-sys`, `rsa`, `glib` y `proc-macro-error`. Registrar también las features que activan cada dependencia.
5. Distinguir dependencias activas, herramientas de compilación y paquetes que solo aparecen en el lockfile. Comprobar las restricciones y correcciones disponibles en fuentes upstream.

**Aceptación:** cada alerta tiene versión, cadena de dependencias, destinos afectados, corrección posible y decisión pendiente identificados. La ausencia en el árbol Windows no se extrapola a Linux ni al lockfile.

**Evidencia:** informes de auditoría, revisión de RustSec y árboles de dependencias adjuntos a la revisión de seguridad.

### T02 · Corregir SQLite / RUSTSEC-2026-0459

- [x] Completar T02. Prioridad: bloqueante.

**Acciones:**

1. Buscar una versión mantenida de SQLx que admita un `libsqlite3-sys` corregido. La revisión inicial encontró 0.30.1 activo y corrección en >=0.39.0; SQLx 0.9.0 por sí solo no resolvía el problema.
2. Preferir una actualización upstream compatible. Si sigue sin existir, preparar una decisión técnica antes de implementar un parche: origen, licencia, mantenimiento, API/ABI, plataformas y forma de retirarlo al existir una solución upstream.
3. No forzar una versión incompatible solo para silenciar el escáner. Si se usa SQLite del sistema, verificar su versión efectiva en todas las plataformas y modificar el empaquetado para garantizar la corrección.
4. Validar consultas, transacciones, migraciones, concurrencia y apertura de bases con WAL en entornos aislados. Si existe una reproducción pública segura, incorporarla como prueba de regresión sin datos reales.

**Aceptación:** el binario usa una implementación de SQLite corregida, con versión efectiva comprobada; desaparece la alerta aplicable y las pruebas de almacenamiento y migración pasan en Windows y Linux.

**Evidencia:** diff de manifiestos/lockfile, origen del parche si existe, versión SQLite en ejecución, auditoría y resultados de pruebas.

### T03 · Resolver RSA / RUSTSEC-2023-0071

- [x] Completar T03. Prioridad: bloqueante hasta determinar y resolver su alcance.

**Acciones:** identificar qué controlador o feature introduce `rsa`; actualizar la cadena a una versión corregida, o retirar de forma soportada la dependencia innecesaria y regenerar el lockfile. Verificar que no se habilitan controladores ajenos a SQLite por omisión. No editar el lockfile manualmente para borrar entradas necesarias.

**Aceptación:** no queda una versión vulnerable en el grafo resuelto auditado. Si el paquete no puede retirarse y solo está en el lockfile, documentar el alcance por destino y elevar la decisión al responsable; mantener el bloqueo hasta resolverla sin alterar silenciosamente la política de auditoría.

**Evidencia:** árboles antes/después, features, lockfile y `cargo audit`.

### T04 · Resolver glib / RUSTSEC-2024-0429 en Linux

- [x] Completar T04. Prioridad: bloqueante para Debian.

**Acciones:** localizar la cadena Tauri/GTK/WebKit que usa glib 0.18.5; evaluar actualizaciones compatibles con la corrección upstream, sin mezclar versiones incompatibles de GTK. Si requiere parche, justificarlo y revisar su mantenimiento como en T02. Validar bucle de eventos, diálogos, apertura/cierre de ventanas y operaciones asíncronas con compilación optimizada en Linux.

**Aceptación:** no queda la implementación afectada en el binario Linux y las pruebas de escritorio y empaquetado pasan. No declarar cerrada esta tarea a partir de una compilación Windows.

**Evidencia:** árbol Linux, versión o parche corregido, auditoría y comprobaciones de la aplicación optimizada.

### T05 · Resolver proc-macro-error / RUSTSEC-2024-0370

- [x] Completar T05.

**Acciones:** identificar el consumidor y buscar una actualización mantenida que lo sustituya. Distinguir el aviso de falta de mantenimiento de una vulnerabilidad explotable. Si upstream impide retirarlo, registrar una excepción revisada con alcance de compilación, responsable, seguimiento upstream, condición de retirada y fecha de revisión.

**Aceptación:** dependencia sustituida, o decisión de mantenimiento explícita y trazable. Las excepciones no afectan los bloqueos de SQLite, RSA o glib.

**Evidencia:** árbol de compilación y actualización o decisión documentada.

### T06 · Cerrar la revisión de seguridad y comprobar las barreras de CI

- [x] Completar T06. Depende de T02–T05.

**Acciones:**

1. Repetir npm y Cargo audit con bases actuales y registrar cualquier alerta nueva. Mantener documentado el override de minimatch y retirarlo únicamente cuando upstream ya resuelva una versión segura.
2. Revisar cambios de capacidades Tauri, CSP, IPC, escapado de rutas SSH, límites de procesos y protocolo de confirmación/respaldo/recuperación si fueron afectados por las correcciones.
3. Ampliar la búsqueda de secretos al contenido rastreado y al historial con una herramienta adecuada. Revisar resultados sin volcar valores sensibles. Ante un secreto real, revocarlo y acordar su retirada del historial.
4. Validar sintaxis de workflows y confirmar que un fallo de auditoría impide aprobar Desktop CI y que Release candidate exige aprobación para el SHA exacto.
5. Actualizar `REVIEW_1_1_0.md` con resultados y decisiones. No convertir «sin alertas conocidas» en garantía de ausencia de vulnerabilidades.

**Aceptación:** ningún hallazgo bloqueante abierto; auditorías y CI coherentes con las decisiones registradas; ningún secreto confirmado sin tratar.

**Evidencia:** informes, revisión del diff y ejecuciones CI.

## Tareas de validación

### T07 · Validación técnica desde una instalación limpia

- [x] Completar T07 en Windows x64.
- [ ] Completar T07 en Debian 12 amd64. **Omitida por el responsable; no es una prueba aprobada.**

**Acciones:** desde un checkout limpio del SHA a validar, ejecutar los comandos de Desktop CI: `npm ci`, formato, lint, tipos, Vitest, compilación Tauri, `cargo fmt --all -- --check`, Clippy con `--locked --features custom-protocol --all-targets -- -D warnings` y pruebas con `--locked --features custom-protocol`. Usar el toolchain declarado. Revalidar alineación de versiones en Tauri, npm, Cargo y lockfiles.

**Aceptación:** todos los controles aplicables pasan en ambos sistemas. Revisar las cinco pruebas Rust omitidas: ejecutar las de credenciales en un almacén aislado y el presupuesto de rendimiento en un entorno registrado, o justificar individualmente por qué no son requisitos de esta entrega. No contarlas como aprobadas sin ejecutarlas.

**Evidencia:** logs, conteo de aprobadas/omitidas, motivos y SHA.

### T08 · Regresión de perfiles, Git, claves y recuperación

- [ ] Completar T08 en Windows. **Configuración, autor y recuperación verificados; las firmas SSH/OpenPGP y el acceso remoto real conservan su evidencia Linux previa, sin atribuirla a Windows.**
- [ ] Completar T08 en Linux. **Ampliación omitida; evidencia previa conservada.**

**Escenarios obligatorios:**

- Identidad global distinta de la asignada: Git devuelve nombre/correo seleccionados; comprobar un commit desde terminal con GitPilot cerrado.
- Reglas anidadas y excepción de repositorio: gana la regla esperada; repositorios nuevos heredan la regla correspondiente.
- Perfil A con clave, cambio a B sin clave: no se reutilizan claves por defecto, agente, certificado ni configuración SSH anterior. Comprobar el comando/configuración efectiva y un intento contra un remoto SSH de prueba controlado.
- Perfil con clave propia: acceso de prueba usa esa clave. Firma SSH y OpenPGP opcional/obligatoria funcionan de forma independiente de la autenticación; verificar firma del commit.
- Claves ausentes y rutas con espacios, comillas, porcentajes y metacaracteres: error seguro o resultado correcto, sin ejecución de contenido de la ruta.
- Cancelar vista previa, cambiar archivos después de revisarla, falta de permisos, lock de Git y fallo parcial: no se aceptan planes vencidos; los cambios previos quedan visibles y recuperables.
- Restauración en orden y edición externa posterior: recuperación verificable sin sobrescribir cambios ajenos. HTTPS conserva su comportamiento documentado.

**Aceptación:** configuración, autor y firmas comprobados con Git CLI; ningún uso accidental de claves de otro perfil en los contextos soportados. Documentar los límites de precedencia externa (`GIT_SSH_COMMAND`, autor/committer por entorno y `git -c`).

**Evidencia:** fixtures temporales reproducibles, resultados y diferencias de configuración sin secretos; ampliar [TESTING.md](../testing/TESTING.md).

### T09 · Revisión visual, interacción y accesibilidad

- [ ] Completar T09 en WebView2/Windows. **Revisión realizada con los límites registrados abajo; no acredita selección efectiva de archivo ni auditoría WCAG completa.**
- [ ] Completar T09 en WebKitGTK/Linux. **Omitida por el responsable.**

**Acciones:** revisar la ventana de 900 × 600, reflujo con zoom/ancho estrecho, navegación por teclado, foco, contraste y movimiento reducido. Comprobar menú vertical con iconos, nombres largos y directorios homónimos con ruta consultable. En asignaciones probar carga, error/reintento, vacío, predeterminado, múltiples raíces, deduplicación y repositorio ausente. Probar seleccionar/cancelar archivos y carpetas, cambiar de perfil y gestionar asignaciones desde Directorios.

**Aceptación:** controles accesibles, sin recortes que impidan operar; textos y estados comprensibles; lista de asignaciones coherente con las reglas guardadas. Datos ficticios en capturas.

**Evidencia:** capturas de ambas plataformas, lista de escenarios y correcciones realizadas.

## Tareas de distribución

### T10 · Integrar, revisar documentación y congelar el SHA

- [x] Completar T10. Depende de T06–T09.

**Acciones:** integrar los commits revisados en `main`; comprobar árbol limpio, archivos permitidos y versiones coherentes. Actualizar CHANGELOG, notas `v1.1.0.md`, revisión de seguridad y pruebas. Retirar el estado de bloqueo solo si la evidencia lo permite. Tras la autorización de push, subir `main` y esperar Desktop CI aprobada para ese SHA exacto.

**Aceptación:** SHA final registrado, documentación fiel al resultado y CI aprobada en Windows con auditorías del lockfile completo. Las nuevas pruebas Linux están omitidas según el ajuste de alcance. Ninguna prueba pendiente se presenta como ejecutada.

**Evidencia:** SHA, commits, revisión y enlace a CI.

### T11 · Construir y verificar los candidatos definitivos

- [x] Completar T11. Depende de T10.

**Acciones:** ejecutar Release candidate desde `main`; descargar `gitpilot-1.1.0-candidate`. Verificar versión y commit de BUILD-INFO, NSIS, Debian, LICENSE, NOTICE y SHA256SUMS. Mantener separados los candidatos locales anteriores, que no deben adjuntarse por error a la release.

**Aceptación:** ambos paquetes provienen del mismo SHA aprobado, todas las sumas coinciden y no falta ningún archivo requerido.

**Evidencia:** enlace a ejecución, BUILD-INFO y manifiesto SHA-256.

### T12 · Instalación, actualización y recuperación con los paquetes reales

- [x] Instalación limpia de NSIS en Windows.
- [x] Actualización Windows desde 1.0.0 con datos sintéticos.
- [ ] Instalación limpia de Debian en un entorno aislado. **Omitida por el responsable.**
- [ ] Actualización Debian desde 1.0.0 con datos sintéticos. **Omitida por el responsable.**
- [x] Reapertura, reinstalación y desinstalación verificadas en Windows.
- [ ] Reapertura, reinstalación y desinstalación en Debian. **Omitidas por el responsable.**

**Acciones:** conservar una copia aislada de datos 1.0.0 y su configuración Git antes de actualizar. Comprobar perfiles, asignaciones, preferencias, historial, migraciones y credenciales de prueba. Verificar conservación de la firma obligatoria de perfiles anteriores, edición de claves y reaplicación confirmada. La instalación por sí sola no debe alterar Git. Comprobar permisos, avisos legales y dependencias del sistema. Documentar recuperación ante fallo de migración; no prometer downgrade sobre una base migrada sin validarlo.

**Aceptación:** sin pérdida de datos ni cambios Git no consentidos; aplicación actualizada operativa con los paquetes de T11; ruta de recuperación probada.

**Evidencia:** versiones antes/después, snapshots aislados, resultados por sistema y hashes de los paquetes usados.

### T13 · Firmar y revisar un borrador de release

- [x] Completar T13. Depende de T12.

**Acciones:** seguir [RELEASING.md](../RELEASING.md). El mantenedor firma el manifiesto con su clave OpenPGP de distribución y exporta únicamente la clave pública. Crear el borrador contra el SHA aprobado, adjuntar paquetes, avisos, procedencia, manifiesto, firma y clave pública. Descargar los adjuntos del borrador y verificar huella, firma y sumas. Anunciar que Windows no lleva Authenticode, salvo que se implemente y valide esa firma antes del manifiesto final.

**Aceptación:** adjuntos completos, firma válida y archivos descargados idénticos a los probados; notas de actualización y límites revisados. Ninguna clave privada ni frase de paso en el repositorio, logs o artefactos.

**Evidencia:** enlace al borrador, huella completa y salida de verificaciones sin secretos.

### T14 · Aprobación y publicación oficial

- [x] Obtener aprobación final del responsable.
- [x] Crear y subir el tag firmado `v1.1.0` al SHA aprobado.
- [x] Comprobar destino/firma del tag y publicar el borrador.

**Aceptación:** todos los bloqueos anteriores cerrados; tag, BUILD-INFO, CI y paquetes identifican el mismo commit; la release pública contiene los archivos verificados. Si algo no coincide, detener la publicación y reconstruir lo necesario.

**Evidencia:** aprobación, SHA del tag y enlace público de la release.

### T15 · Comprobación posterior y cierre

- [x] Descargar los archivos desde la release pública y verificar firma y SHA-256.
- [x] Realizar una comprobación de instalación/apertura con el paquete público en un entorno aislado.
- [x] Revisar enlaces, notas y requisitos publicados; comprobar que apuntan a 1.1.0.
- [x] Actualizar ROADMAP, TESTING y la revisión de seguridad con el cierre real.
- [x] Registrar incidencias restantes y seguimiento de excepciones aprobadas con responsable y fecha.

**Aceptación:** descarga pública utilizable y verificable; plan cerrado con evidencia. Ante un fallo de seguridad o actualización, registrar el incidente, avisar al responsable y decidir retirada o versión correctiva; no reemplazar silenciosamente un paquete publicado.

## Registro de ejecución

Agregar una fila al avanzar o bloquear cada tarea. Guardar logs voluminosos como artefactos y enlazarlos; excluir secretos y datos personales.

| Tarea | Estado | Responsable | Fecha | SHA / sistema | Evidencia | Bloqueo / siguiente acción |
| --- | --- | --- | --- | --- | --- | --- |
| T01–T06 | Verificada | Codex | 2026-11-12 | e2bc12a / Windows y lockfile completo | CI 43331051369; REVIEW_1_1_0.md y ADR-005 | Mantener revisión de backports |
| T07–T08 | CI Windows verificada; límites T08 registrados | Codex | 2026-11-12 | e2bc12a / Windows; evidencia Linux previa | CI 43331051369 y REVIEW_1_1_0.md | No atribuir firmas/remoto reales a Windows |
| T09 | Parcial; Linux omitido | Responsable / Codex | 2026-11-12 | WebView2 QA | REVIEW_1_1_0.md | Conservar límites observados |
| T10–T15 | Distribución verificada | Responsable / Codex | 2026-11-12 | e2bc12a / tag v1.1.0 | Candidato 43332119582 y comprobación pública 43334934682 | Release publicada; conservar límites T08–T09 |

Estados: Pendiente, En curso, Bloqueada, Verificada. Una tarea termina en Verificada, no solo con un commit.

## Candidato definitivo y publicación

El SHA congelado del producto es `e2bc12aaf124650812aeee504f04465af3183925`. [Desktop CI](https://github.com/JhosepAC/gitpilot/actions/runs/43331051369) aprobó formato, lint, tipos, cuatro pruebas frontend, compilación, Clippy, 44 pruebas Rust habilitadas y auditorías. Las cinco pruebas especiales conservan su ejecución local separada. Gitleaks revisó 85 commits hasta este SHA sin detectar secretos. [Release candidate](https://github.com/JhosepAC/gitpilot/actions/runs/43332119582) compiló NSIS y Debian desde ese mismo commit y aprobó instalación limpia, actualización desde 1.0.0, migración, integridad, conservación de perfiles/directorios/reglas, reapertura, reinstalación, desinstalación y ausencia de cambios Git no consentidos en Windows.

Se verificaron las seis entradas de SHA256SUMS y BUILD-INFO al descargar el candidato, los ocho adjuntos iniciales del borrador y los nueve archivos definitivos de la [release pública](https://github.com/JhosepAC/gitpilot/releases/tag/v1.1.0). La firma añadida al borrador también se descargó y verificó antes de publicar. Los commits posteriores que solo registran esta evidencia no cambian el SHA de los paquetes ni requieren reconstruirlos.

T08 distingue la verificación Windows de configuración efectiva/autor/recuperación de las pruebas reales de remoto SSH y firmas realizadas previamente en Linux. T09 Windows verificó navegación vertical, directorios homónimos, rutas largas, tres reglas deduplicadas, predeterminado, carga, vacío, repositorio ausente, error y reintento recuperado, foco, navegación con Enter y reflujo 100–200%. El selector nativo de clave se abrió y canceló; la selección efectiva de un archivo no quedó acreditada por una limitación de control de la ventana modal. No se atribuye una certificación WCAG ni cobertura exhaustiva a esta revisión. Estas comprobaciones pendientes permanecen visibles, sin confundirlas con fallos de producto encontrados.

El mantenedor desbloqueó localmente la clave OpenPGP y se firmó el manifiesto con `47B10C26AF8BC9615DE39C5813EF6F27474B5E0F`. Se verificaron firma y sumas del borrador; el tag SSH firmado `v1.1.0` se comprobó localmente y en el remoto sobre el SHA congelado. La [release](https://github.com/JhosepAC/gitpilot/releases/tag/v1.1.0) se publicó como última versión el 2026-11-12 a las 19:09 America/Bogota (2026-11-13 00:09 UTC), con nueve adjuntos. Su descarga pública, firma, clave y sumas se volvieron a comprobar. [La prueba de instalación del paquete público](https://github.com/JhosepAC/gitpilot/actions/runs/43334934682) pasó: autenticó los nueve adjuntos, comprobó la procedencia frente al tag y repitió instalación, actualización, migración, reapertura, reinstalación y desinstalación en Windows desechable. La autorización condicional del responsable y su intervención de desbloqueo quedaron cumplidas, sin cambiar la clave de distribución.

| Tarea | Estado actualizado | Evidencia / siguiente acción |
| --- | --- | --- |
| T01–T07 | Auditorías y CI Windows verificadas | CI 43331051369; T07 Linux omitida |
| T08–T09 | Cobertura y límites registrados | TESTING.md y revisión de seguridad; no marcar casos no ejecutados |
| T10–T12 | SHA y candidato verificados; instalación Windows aprobada | Candidato 43332119582; instalación Debian omitida |
| T13 | Verificada | Nueve adjuntos, firma y bytes del borrador comprobados |
| T14 | Verificada | Tag SSH firmado y release pública sobre e2bc12a |
| T15 | Verificada | Descarga autenticada e instalación pública aprobadas: Actions 43334934682 |

## Seguimiento de límites y mantenimiento

Registro del 2026-11-12, responsable: mantenedor de GitPilot. Los backports deben revisarse y retirarse cuando upstream permita hacerlo, antes del 2026-12-12 o de la próxima versión, lo que ocurra primero (ADR-005). T08–T09 conservan explícitos los límites de comprobación Windows y los casos manuales no acreditados; ampliar esa cobertura en la próxima revisión de escritorio sin presentar la evidencia Linux como Windows. La omisión de nuevas pruebas Linux es una decisión del responsable de esta entrega. No quedan vulnerabilidades conocidas sin tratar en las auditorías registradas; no se promete ausencia absoluta de vulnerabilidades. Windows conserva la limitación de distribución sin Authenticode, con manifiesto OpenPGP verificado.
