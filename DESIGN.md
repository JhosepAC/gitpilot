---
name: GitPilot
description: "Sistema visual de Identidades para el escritorio de GitPilot."
colors:
  primary: "#f5511e"
  accent-hover: "#ff7045"
  canvas: "#101010"
  rail: "#141414"
  surface: "#191919"
  selected: "#242424"
  line: "#303030"
  control-line: "#707070"
  selection-line: "#767676"
  text: "#f1f1f1"
  muted: "#adadad"
  danger: "#ff9e94"
typography:
  brand:
    fontFamily: "system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 670
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 660
    lineHeight: 1.2
    letterSpacing: "-0.03em"
  title:
    fontFamily: "system-ui, sans-serif"
    fontSize: "1.1875rem"
    fontWeight: 640
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  section:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 620
  profile-label:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 630
  value:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.9375rem"
  body:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.875rem"
  button:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 610
  label:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 600
  caption:
    fontFamily: "system-ui, sans-serif"
    fontSize: "0.75rem"
rounded:
  control: "0.375rem"
  card: "0.75rem"
spacing:
  action-gap: "0.5rem"
  section-gap: "1rem"
  panel-inline: "1.75rem"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.canvas}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "0.4rem 0.75rem"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "0.4rem 0.75rem"
  button-danger:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.danger}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "0.4rem 0.75rem"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0.4rem 0.625rem"
  profile-current:
    backgroundColor: "{colors.selected}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "0.5rem 0.625rem"
  commit-author:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.card}"
    padding: "0.875rem 1rem"
---

# Sistema de diseño de GitPilot

## Overview

**Registro de perfiles.** La lista de identidades y el detalle del autor forman una sola superficie de trabajo. La lista permite reconocer perfiles por nombre y correo. El detalle da prioridad al nombre y al correo que identifica Git, con el uso local y las acciones debajo.

Los negros mate separan regiones por tono y las divisiones finas mantienen la lectura continua. La tipografía de sistema y los controles compactos mantienen visible el contexto de trabajo.

## Colors

- **Naranja Git (`primary`).** Marca, acción principal y foco de teclado. La variante `accent-hover` aclara la acción al pasar el puntero o pulsarla. El texto de la acción principal usa el negro del lienzo.
- **Negros mate (`canvas`, `rail`, `surface`, `selected`).** El lienzo sostiene el detalle. La barra lateral se distingue por un tono propio y la tarjeta del autor y los controles usan la superficie. El perfil activo tiene un relleno más claro.
- **Bordes neutros (`line`, `control-line`, `selection-line`).** El primero divide regiones, el segundo delimita controles con al menos 3:1 sobre las superficies del tema y el tercero marca el perfil activo (`#767676`).
- **Texto (`text`, `muted`).** El tono claro identifica títulos, valores y acciones. El gris se usa en descripciones, etiquetas secundarias y metadatos.
- **Peligro (`danger`).** El rojo claro acompaña el texto de alerta y el verbo de la acción destructiva.

**Selección y foco.** La fila activa conserva su borde neutro al perder el foco. Al navegar con teclado, el contorno naranja distingue el destino actual.

## Typography

La interfaz usa `system-ui, sans-serif`. El título de pantalla (1.5 rem, peso 660) encabeza la lectura. El nombre del perfil y los títulos de formulario (1.1875 rem, peso 640) organizan el detalle. Los encabezados de sección (0.875 rem, peso 620) agrupan los datos del autor y su uso local.

Los valores del autor (0.9375 rem) dominan sobre las etiquetas y el correo en la lista (0.75 rem). Los nombres de perfil (0.875 rem, peso 630) destacan en la barra lateral. Los botones y las etiquetas de formulario (0.8125 rem) usan peso medio y el contador de perfiles usa cifras tabulares.

## Layout

**Ventana fija.** El área de contenido mide 900 × 600 píxeles lógicos. El marco y la barra de título del sistema alojan los controles de minimizar y cerrar. La ventana carece de redimensionado y maximización.

**Lista y detalle.** La barra lateral de 14.5 rem ocupa toda la altura. La columna derecha reúne el encabezado y el detalle, con 1.75 rem de margen interior lateral. En la vista de dos columnas, la lista y el detalle se desplazan por separado. Las acciones se separan por 0.5 rem y pueden pasar a la línea siguiente.

**Reflujo con zoom.** Cuando el ancho disponible en el área de contenido llega a 45 rem, se apilan encabezado, perfiles y detalle. La lista mantiene desplazamiento propio con un máximo de 10 rem y el detalle pasa al desplazamiento general. A 28 rem, el nombre y el correo del autor se apilan y la altura mínima de los botones aumenta a 2.75 rem. Los nombres y correos largos de la lista y el detalle pueden partir línea para conservar su lectura.

## Elevation & Depth

La jerarquía se apoya en fondos opacos, diferencias de tono y líneas de 1 px. La tarjeta del autor gana presencia por el fondo y el espacio interior. Los avisos comparten el tono de superficie y aparecen junto al contenido al que se refieren.

## Shapes

Los controles, las filas de perfil y los avisos tienen esquinas de 0.375 rem. La tarjeta del autor usa 0.75 rem y un pequeño círculo de contorno naranja junto al encabezado. Las divisiones rectas separan barra lateral, encabezado y acciones del detalle.

## Components

### Botones

La acción principal usa fondo naranja y texto oscuro. Los botones secundarios tienen fondo de superficie y borde neutro. La acción destructiva mantiene esa forma y usa el tono de peligro en el texto. La altura mínima habitual es 2.125 rem. El fondo transiciona durante 120 ms con `ease-out` y la pulsación desplaza el botón 1 px. La preferencia de movimiento reducido desactiva ambos efectos.

### Campos

La etiqueta queda encima del control. Los campos usan fondo de superficie, borde neutro, texto de 0.875 rem y altura mínima de 2.25 rem. El cursor de texto es naranja y el ejemplo usa el gris de metadatos. El formulario limita su ancho a 29 rem.

### Lista de perfiles

Cada fila muestra nombre, correo y, cuando corresponde, la etiqueta «Predeterminada». El botón de la fila actual combina fondo elevado en tono, borde neutro y `aria-current`. La selección de un perfil lleva el foco al detalle, donde un subrayado naranja identifica el encabezado cuando se muestra el foco de teclado. Los botones y campos usan un contorno naranja de 2 px, separado otros 2 px del borde.

### Tarjeta de autor y estados del detalle

«Autor del commit» agrupa nombre y correo en una cuadrícula de proporción 2:3 que se apila cuando se estrecha el contenido. El detalle aloja también el formulario, la confirmación de eliminación y el estado vacío. Las actualizaciones de progreso y resultado se anuncian con `status` y los errores con `alert`.

## Do's and Don'ts

- Reserva el relleno naranja para la acción principal de cada estado. Mantén neutro el borde del perfil activo.
- Conserva las señales de foco cuando coincidan con un estado seleccionado o con un aviso.
- Deja que los textos largos y los controles pasen a otra línea en el área de contenido. Mantén el acceso por teclado al detalle y a sus acciones durante el reflujo.
