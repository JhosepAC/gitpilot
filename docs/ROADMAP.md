# Plan de desarrollo de GitPilot

Alcance de lanzamiento · Windows y Linux x86_64. macOS queda como ampliación futura.

## Resultado buscado

Un desarrollador registra sus perfiles una vez, asigna carpetas y excepciones por repositorio y puede comprobar qué nombre y correo usará Git para el autor y el committer. Tras confirmar un cambio, Git conserva la configuración en terminales y editores con GitPilot cerrado, dentro de los contextos Git verificados. Las cuentas remotas y la firma se incorporan como capacidades independientes.

| Documento | Decisiones y criterios |
| --- | --- |
| [Producto](product/PRD.md) | Actores, casos de uso, ocho módulos y aceptación del MVP |
| [Arquitectura](architecture/ARCHITECTURE.md) y [datos](architecture/DATABASE.md) | Capas, IPC, dependencias, esquema y persistencia |
| [GitHub](architecture/GITHUB.md) | Registro, permisos, autorización, credenciales y entregas de fase 6 |
| [Resolución](architecture/IDENTITY_RESOLUTION.md) y [configuración Git](architecture/GIT_CONFIGURATION.md) | Identidad esperada, estado efectivo, escrituras y recuperación |
| [Seguridad](security/SECURITY.md) y [autenticación](security/AUTHENTICATION.md) | Amenazas, límites, SSH, HTTPS, firma y OAuth |
| [UX](ux/UX.md) | Flujos, dirección visual y validación |
| [Pruebas](testing/TESTING.md) | Escenarios, CI, cobertura y publicación |
| [Fase 0](testing/PHASE_0.md) | Prueba reproducible, resultados por plataforma y condiciones pendientes |
| [ADRs](decisions/README.md) | Decisiones iniciales y preguntas abiertas |

## Entregas y condiciones de salida

Cada fila se descompone en PR pequeños y ordenados. El hito puede cerrarse cuando cumple su evidencia de salida y los criterios de los documentos relacionados.

| Orden | Entrega | Trabajo | Evidencia de salida |
| --- | --- | --- | --- |
| 0 | Especificación y viabilidad | Revisar el alcance, preparar wireframes del flujo principal y ejecutar la prueba técnica aislada de Git de [configuración](architecture/GIT_CONFIGURATION.md). Fijar Git mínimo, alcance por tipo de repositorio y estrategia de recuperación. | Especificación revisada, resultados por sistema y decisiones registradas. Resolver antes de construir pantallas que dependan de ese comportamiento. |
| 1 | Infraestructura | Crear `apps/desktop`, Tauri 2, React/Vite, Tailwind, Rust y SQLx. Añadir migración, IPC tipado, errores y logs filtrados, capacidades mínimas y CI. Documentar requisitos y comandos de desarrollo. | App abre en Windows y Linux, base temporal migra y CI compila y prueba. |
| 2 | Identidades | Validar y persistir perfiles. Incorporar CRUD, duplicado, valor predeterminado, visualización de usos e importación voluntaria de la identidad existente. | Perfiles sobreviven a reinicios. Eliminación segura con dependencias y perfiles con correos iguales. |
| 3 | Directorios y repositorios | Registrar raíces, escanear bajo demanda, excluir rutas, mostrar resultados parciales, asignar reglas y excepciones. Incorporar resolución en lectura. | Repositorios temporales y anidados detectados dentro de las raíces. Precedencia, procedencia y conflictos visibles en UI. |
| 4 | Configuración nativa y v0.1.0 | Conectar vista previa, aplicación, verificación, auditoría y recuperación. Completar onboarding y diagnóstico básico. Preparar instaladores para Windows y Linux. | Todos los criterios M1–M6 pasan, incluida la creación de commits fuera de GitPilot. |
| 5 | SSH, HTTPS, firma e Inspector | Diagnóstico de autenticación, claves existentes, firma, remotos, rama y estado. Añadir pre-commit voluntario con advertencias y bloqueo optativo. | Diagnóstico y reversión probados sin mezclar identidad de commit, firma y acceso. |
| 6 | GitHub | Elegir tipo de app y permisos. Conectar y revocar varias cuentas, consultar repositorios, organizaciones, PR e issues autorizados, y asociar remotos. | Flujo de escritorio y almacenes seguros comprobados en Windows y Linux. Pruebas de permisos, revocación y límites de API. |
| 7 | Beta | Completar preferencias, importación/exportación, actividad, idioma, accesibilidad, rendimiento y estados de error. Recoger feedback de uso. | Flujos principales revisados en cada plataforma, presupuestos de rendimiento medidos y hallazgos críticos cerrados. |
| 8 | v1.0.0 | Publicar desde un commit verificado de `main`. Preparar firma según plataforma, checksums, política de seguridad, licencia elegida, changelog, README ES/EN y guía de contribución. | Instalación y actualización comprobadas desde los artefactos publicados, notas de compatibilidad y recuperación accesibles. |

## Trunk-based development

- `main` es la única rama permanente. Protegerla cuando exista el primer commit en el remoto, exigir revisión y CI, restringir push directo y force-push. Publicar por tags de versión sobre commits verificados de `main`.
- Abrir ramas desde `main` con un único objetivo. Nombrarlas por el cambio, por ejemplo `feat/identity-validation` o `fix/credential-recovery`. Objetivo de integración de un día laborable, con dos días como umbral para dividir el trabajo. Actualizar frente al tronco y eliminar la rama al integrar.
- Integrar en orden contratos, implementación probada y exposición del flujo. Si una parte debe aterrizar antes, mantenerla inaccesible o detrás de una bandera con responsable y fecha de retirada. Las migraciones y los flujos habilitados deben seguir funcionando en cada integración.
- El PR indica el alcance, el requisito, los riesgos, la evidencia y los cambios de documentación. El mantenedor revisa los cambios de Git, seguridad y migraciones. Si falla una validación crítica en `main`, corregir o revertir el cambio con prioridad.
- Mantener fuera del historial las skills, instrucciones, prompts, índices y notas locales de asistentes. Usar `.gitignore` y revisar `git diff --cached --name-only` antes de cada commit. Agregar en CI una comprobación de rutas rastreadas que cubra los patrones prohibidos, incluso ante un agregado forzado. La documentación de producto y desarrollo vive en `docs/`. Los commits y el push se realizan con autorización del responsable.

## Estado de la fase 0

Fase cerrada con los [wireframes](ux/WIREFRAMES.md), los ADR y la [evidencia de Windows y Linux](testing/PHASE_0.md). La implementación se limita a infraestructura en fase 1. La gestión de identidades comienza en fase 2.

## Estado de la fase 1

Infraestructura implementada y verificada en Windows y Linux. La aplicación abre su ventana, aplica la primera migración y confirma el estado de SQLite mediante IPC. El [README](../README.md) documenta la ejecución y las [pruebas](testing/TESTING.md#evidencia-de-fase-1) registran la validación local. La fase 2 comienza con perfiles de identidad persistidos.

## Estado de la fase 2

La pantalla Identidades permite administrar perfiles persistidos, elegir un predeterminado e importar la identidad global de Git mediante un borrador revisable. Las [pruebas](testing/TESTING.md#evidencia-de-fase-2) registran la validación en Windows y Linux. La fase 3 incorpora directorios, repositorios y asignaciones de identidad.

## Estado de la fase 3

La pantalla Directorios registra raíces, descubre repositorios bajo demanda y admite exclusiones. Conserva los resultados anteriores cuando el escaneo es parcial. La identidad esperada se calcula con la excepción del repositorio, la regla del ancestro más específico o el predeterminado dentro del inventario. El diagnóstico muestra las discrepancias, el origen de cada valor de Git y el autor y committer que Git devuelve para el contexto de la aplicación. Las reglas se guardan en SQLite. La fase 4 materializará los cambios de configuración previa revisión y confirmación. [Validaciones](testing/TESTING.md#evidencia-de-fase-3)

## Estado de la fase 4

Fase completada para los candidatos locales de v0.1.0. Incluye Inicio, vista previa, escritura global y local confirmada, respaldo cifrado, Actividad y restauración. Los repositorios ordinarios nuevos heredan las reglas con GitPilot cerrado. Los casos M1–M6 se verificaron con datos temporales y los paquetes NSIS y Debian pasaron instalación y desinstalación. La publicación y la firma de distribución quedan pendientes. La fase 5 empieza con el diagnóstico de SSH, HTTPS y firma. [Validaciones](testing/TESTING.md#evidencia-de-cierre-de-fase-4)

## Estado de la fase 5

El Inspector muestra la rama, el estado del índice y las modificaciones rastreadas sin preparar cuando puede comprobarlas de forma aislada. Ofrece lectura HTTPS anónima o con credenciales existentes, SSH directo o con una ruta configurada y firma de desafíos mediante SSH u OpenPGP. El hook pre-commit voluntario admite aviso, bloqueo y retirada protegida. Los formatos de credenciales, directivas SSH y referencias de firma compatibles se detallan en la especificación. [Alcance del diagnóstico](security/AUTHENTICATION.md)

## Estado de la fase 6

GitPilot administra cuentas con fotos de perfil, renovación y desconexión local. La fase 6 consulta instalaciones, repositorios autorizados, pull requests e issues. También vincula un repositorio autorizado con el destino de lectura de un remoto del inventario. La fase 7 prepara la beta con preferencias, portabilidad de datos y revisión de accesibilidad y rendimiento. [Especificación de GitHub](architecture/GITHUB.md)
