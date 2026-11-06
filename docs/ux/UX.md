# Experiencia de escritorio

## Dirección de diseño

Minimalista y moderna, con GitHub Desktop y GitHub Primer como referencias para navegación, listados de repositorios, estados y lectura de cambios. Usar el vocabulario de Git donde aporte precisión y acompañarlo con explicaciones al mostrar precedencia, origen o restauración. [GitHub Primer](https://primer.style/product/)

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

## Criterios visuales y de accesibilidad

- Barra lateral con selección visible y panel de trabajo estable al cambiar de repositorio. En ventanas estrechas, plegar navegación y reducir columnas conservando acceso al detalle. Usar superficies neutras, un acento de acción/selección y estados semánticos con texto e icono.
- Jerarquía de tipografía de sistema con escala y espaciado consistentes. Usar monoespaciada para rutas, claves y comandos donde mejore la lectura. Las rutas truncadas conservan una forma accesible de consultar y copiar la ruta completa.
- Temas claro, oscuro y del sistema. Aplicar los criterios WCAG 2.2 AA que correspondan a la interfaz, con foco visible, orden de tabulación, teclado completo, contraste, nombres accesibles, retorno de foco y anuncios de estados importantes. Verificar aumento de texto y preferencias de movimiento. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- Diseñar vacío, carga, error, resultado parcial, falta de permisos, conflicto y recuperación. Mostrar qué ocurrió y la acción disponible. Validar en ventana compacta, habitual y maximizada, con datos largos, navegación por teclado y los webviews de las plataformas objetivo.
