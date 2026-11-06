# Wireframes del flujo principal

Baja fidelidad · datos ficticios · modo Operate. Los cuadros representan jerarquía, contenido y decisiones. La [dirección visual](UX.md) se aplica al prototipar la interfaz.

## 1. Elegir carpeta e identidad

```text
 GitPilot            Directorios / Añadir carpeta
 ─────────────────┬────────────────────────────────────────────────────────
 Resumen          │ ¿Qué identidad quieres usar en esta carpeta?
 Identidades      │
 Directorios  ●   │ Carpeta     ~/Proyectos/Empresa           [Cambiar…]
 Repositorios     │ Identidad   Empresa · dev@empresa.test    [Elegir  ▾]
 Actividad        │                                             Crear perfil
 Ajustes          │
                  │ Escaneo completado · 3 repositorios · 1 ruta excluida
                  │ api              Coincide con la regla
                  │ web              Configuración local diferente
                  │ api-prueba       Worktree vinculado · Solo diagnóstico
                  │
                  │ [Cancelar]                             [Revisar cambios]
```

En el primer inicio, mostrar **Comprobar Git** antes de este paso. Si Git falta, indicar la versión mínima y ofrecer **Volver a comprobar**. Durante el escaneo, mostrar progreso, resultados parciales y **Cancelar escaneo**. Los borradores se pueden retomar. Las exclusiones del escaneo se explican junto al resumen.

## 2. Revisar la propuesta y su alcance

```text
 Directorios / Empresa / Vista previa
 ──────────────────────────────────────────────────────────────────────────
 Usar Empresa en repositorios ordinarios de esta carpeta
 El nombre y el correo se aplicarán también a los repositorios nuevos.

 ARCHIVOS                     IDENTIDAD PREVISTA
 ~/.gitconfig                 api              dev@empresa.test
 Reglas de GitPilot           web              Conserva dev@personal.test
                              api-prueba       Solo diagnóstico

 Se creará un respaldo antes de aplicar.          [Ver cambios por archivo]
 1 exclusión del escaneo puede seguir heredando la regla.

 [Volver]                                                  [Aplicar regla]
```

Cada fila permite consultar el origen, el alcance y la identidad prevista. La acción de aplicar indica el cambio exacto y confirma el plan mostrado. Si cambia un archivo desde la vista previa, marcar la propuesta como vencida y reemplazar la acción principal por **Actualizar vista previa**. Si un destino conocido quedaría afectado fuera del alcance admitido, mostrar el motivo y el recurso que requiere revisión.

## 3. Resolver una discrepancia en un repositorio

```text
 Repositorios / web                              Conflicto de identidad
 ──────────────────────────────────────────────────────────────────────────
 Esperada       Empresa · dev@empresa.test       Regla de directorio
 Observada      Empresa · dev@personal.test      .git/config · Local

 Nombre         Empresa                          Manifiesto de GitPilot
 Correo         dev@personal.test                .git/config

 Autor observado       Empresa <dev@personal.test>
 Committer observado   Empresa <dev@personal.test>

 [Conservar configuración actual]          [Revisar cambio de identidad]
                                                Elegir excepción…
```

**Conservar** registra la decisión para la revisión actual. **Revisar cambio de identidad** abre otra vista previa centrada en el archivo y las claves locales que se tocarían. **Elegir excepción** modifica la intención de GitPilot y presenta una nueva vista previa. Si el valor proviene de una fuente incompatible, mostrar el origen y mantener disponibles las acciones de lectura.

## 4. Verificar o recuperar

```text
 Actividad / Regla Empresa
 ──────────────────────────────────────────────────────────────────────────
 ✓ Regla verificada                           1 conflicto conservado
 Git confirmó la identidad prevista en los repositorios compatibles.

 Comprobar desde una terminal externa
 git config user.email                                         [Copiar]

 [Ver repositorios]                                      [Revisar restauración]
```

| Estado | Contenido y acción |
| --- | --- |
| Aplicando o verificando | Mostrar la etapa y bloquear una segunda confirmación. Anunciar el avance y mantener accesible la navegación de lectura. |
| Verificado | Mostrar alcance, excepciones y conflictos conservados. Ofrecer acceso al diagnóstico y a la revisión de restauración. |
| Fallo restaurado | Informar el fallo y confirmar el estado anterior tras verificar con Git. Ofrecer volver a inspeccionar. |
| Recuperación pendiente | Nombrar el archivo y la causa, conservar el respaldo y bloquear escrituras sobre ese destino. Ofrecer revisar diferencias y pasos de recuperación. |

En ventana estrecha, plegar la barra lateral y apilar archivos e identidades. Mantener visible el destino de la acción. Al cambiar de paso, enfocar el encabezado. Al volver de un diálogo, devolver el foco al control que lo abrió. Los estados usan texto y un icono. Las transiciones dan respuesta inmediata y funcionan con movimiento reducido.
