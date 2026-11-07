# Experiencia de escritorio

## Dirección de diseño

Interfaz de trabajo minimalista con una escala de negros mate y naranja Git `#f5511e` para el foco, la marca y la acción principal. Usar el vocabulario de Git donde aporte precisión y acompañarlo con explicaciones al mostrar precedencia, origen o restauración. Los [tokens del rediseño](../../DESIGN.md) fijan los colores, la tipografía y la composición.

- **`impeccable`** guía el modo Operate, la jerarquía, los flujos, la densidad y la revisión visual. Preparar wireframes y estados con datos ficticios antes de construir. Al implementar cada superficie, revisar el resultado renderizado en tamaños reales, ejecutar la auditoría aplicable y documentar las decisiones visuales como documentación de producto.
- **`apple-design`** guía la respuesta inmediata, la consistencia espacial, la tipografía de sistema y el movimiento funcional. Mostrar cambios de estado en cuanto se recibe la interacción. Mantener las transiciones breves e interrumpibles y dar una alternativa con movimiento reducido. Reservar transparencias para capas que ayuden a distinguir superficies y ofrecer fondo sólido cuando lo requieran legibilidad o preferencias.
- Mantener skills, configuraciones de herramientas, prompts y artefactos de trabajo de asistentes en local. La especificación visual y los componentes del producto forman parte de la documentación y el código compartidos.

## Estructura y pantallas

| Momento | Superficie | Contenido y acción principal |
| --- | --- | --- |
| MVP | Bienvenida y configuración | Detectar Git, explicar versión y configuración observada, crear o importar primera identidad, elegir carpetas, revisar propuesta, aplicar y verificar. Permitir salir y retomar antes de aplicar. |
| MVP | Resumen | Repositorios sin asignación, conflictos y operaciones pendientes. Enlazar a cada caso desde listados compactos. |
| MVP | Identidades y detalle | Nombre visible, nombre Git, correo y usos. Crear, editar, duplicar, elegir predeterminado y resolver dependencias antes de eliminar. |
| MVP | Directorios | Añadir raíz, asignar perfil, mostrar exclusiones y progreso. Indicar el alcance nativo de la regla con una vista previa. |
| MVP | Repositorios y detalle | Listado con identidad esperada y estado. En el detalle, comparación por campo con valores y orígenes, regla ganadora, excepciones y acciones válidas. |
| MVP | Actividad y ajustes | Ver aplicación, decisiones y restauraciones. Gestionar respaldos, preferencias de escaneo y tema. |
| Posterior | Autenticación, Inspector y GitHub | Claves y aliases existentes, diagnóstico SSH/HTTPS, firma, hooks optativos, cuentas y recursos autorizados. La inspección SSH se incorpora al asistente cuando ese módulo esté disponible. |

El flujo central es **Carpeta → Identidad → Vista previa → Aplicar → Verificado**. La vista previa enumera archivos, cambios de nombre/correo, ámbito, repositorios con conflictos y efecto en repositorios futuros. Un conflicto ofrece **Conservar configuración actual** o **Revisar y aplicar la identidad esperada** cuando existe un plan viable. Un fallo diferencia **Restaurado** de **Recuperación pendiente**, con las acciones apropiadas en cada estado.

Los [wireframes de fase 0](WIREFRAMES.md) concretan los pasos y los estados de conflicto, vista previa vencida y recuperación.

**Identidades.** Una lista lateral muestra los perfiles y la selección predeterminada. El panel principal reúne el nombre y correo en «Autor del commit», con el uso local y las acciones debajo. Crear, editar e importar comparten un formulario. La importación empieza como borrador revisable y la eliminación se confirma en el panel. El uso local indica el predeterminado y el número de asignaciones.

**Directorios.** La navegación lateral comparte la ventana fija con Identidades. Registrar una raíz requiere su ruta absoluta y el escaneo empieza al pulsar «Escanear». El panel informa del resultado y permite gestionar exclusiones y asignaciones. El detalle de cada repositorio distingue la identidad esperada de la configuración observada en Git y deja las rutas internas y los valores de autor y committer en secciones desplegables.

**Vista previa de Git.** Se abre en el panel de Directorios, con el destino y el manifiesto primero. Los repositorios con discrepancias aparecen antes que el resto. Se puede desplegar la comparación por campo y el patrón de cada regla. Volver al inventario conserva la selección y devuelve el foco al botón de entrada.

**Aplicación y restauración.** Una vista previa verificable ofrece «Revisar aplicación». La confirmación en el mismo panel repite el archivo global. Una vez verificada, la aplicación muestra el destino y permite confirmar la restauración. «Recuperación pendiente» identifica el archivo cuyo estado requiere revisión.

**Inicio y Actividad.** Inicio muestra la versión de Git y permite retomar los pasos para guardar perfiles, registrar directorios y revisar la configuración. Actividad ordena decisiones y operaciones recientes por fecha, distingue el ámbito global del repositorio y abre el destino de cada operación activa. El diagnóstico ofrece conservar una discrepancia y, cuando corresponde, abrir la vista previa del archivo local.

**Inspector de Git.** Desde el detalle de un repositorio presente, el Inspector abre la rama y el índice primero. Luego muestra los remotos y separa el acceso SSH, el acceso HTTPS y la firma de commits. Volver conserva la selección y devuelve el foco al botón de entrada.

**Comprobaciones y pre-commit.** El Inspector distingue lectura HTTPS anónima y con credenciales, SSH directo y configurado, y las comprobaciones de firma. Antes de ejecutar muestra la modalidad, el destino y, cuando corresponde, el salto SSH o la referencia OpenPGP. El foco vuelve al control de origen tras cancelar o terminar. La instalación del hook muestra el archivo, la identidad revisada y si emitirá un aviso o bloqueará el commit. La retirada tiene su propia confirmación y queda registrada en Actividad.

**GitHub.** La lista muestra las cuentas por foto y login y distingue la selección con un fondo mate. La inicial reemplaza la foto si falla la carga. El detalle presenta el ID remoto y las fechas de comprobación y vencimiento. Durante la autorización se muestran el código seleccionable, el destino del navegador y la acción de cancelar. La desconexión confirma qué cuenta se retirará del equipo y muestra la acción remota por separado. Los estados de espera y error se anuncian.

**Instalaciones y repositorios de GitHub.** El explorador ocupa el panel de detalle y conserva la lista de cuentas. Cada fila de instalación muestra su propietario y alcance. Los repositorios aparecen por nombre completo, visibilidad y estado de archivo. Las acciones para volver, cambiar de página y actualizar conservan el contexto de la cuenta. Una lista vacía explica dónde revisar el acceso.

**Actividad y vínculos de GitHub.** El detalle del repositorio agrupa pull requests, issues y vínculo local. Cada fila de actividad muestra título, número y estado. La confirmación del vínculo repite el repositorio local, el remoto y el destino autorizado. Un cambio de destino muestra «Requiere revisión» en el panel de vínculo y en el Inspector. El Inspector permite retirar la asociación guardada desde el repositorio local.

**Ventana.** El área de trabajo es fija de 900 × 600 píxeles lógicos, con minimizar y cerrar mediante los controles nativos. La lista y el detalle se desplazan de forma independiente. El diseño reorganiza los paneles cuando el zoom reduce el espacio de contenido.

## Criterios visuales y de accesibilidad

- Barra lateral con selección visible y panel de trabajo estable al cambiar de perfil. Cuando disminuye el ancho de contenido, apilar la lista y el detalle. Usar superficies neutras y estados semánticos con texto.
- Jerarquía de tipografía de sistema con escala y espaciado consistentes. Usar monoespaciada para rutas, claves y comandos donde mejore la lectura. Las rutas truncadas conservan una forma accesible de consultar y copiar la ruta completa.
- Tema oscuro mate. Aplicar los criterios WCAG 2.2 AA que correspondan a la interfaz, con foco visible, orden de tabulación, teclado completo, contraste, nombres accesibles, retorno de foco y anuncios de estados importantes. Verificar aumento de texto y preferencias de movimiento. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- Diseñar vacío, carga, error, resultado parcial, falta de permisos, conflicto y recuperación. Mostrar qué ocurrió y la acción disponible. Validar la ventana fija y el reflujo con zoom, datos largos, navegación por teclado y los webviews de las plataformas objetivo.
