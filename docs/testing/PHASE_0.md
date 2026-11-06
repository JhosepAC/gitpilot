# Fase 0 · Especificación y viabilidad

Estado · cerrada para Windows y Linux. El alcance, los wireframes, el protocolo de recuperación y las pruebas de Git quedan registrados aquí y en los ADR. macOS se evaluará como ampliación futura.

## Reproducir la prueba

Ejecutar desde el repositorio con Python 3.9 o posterior y Git en PATH.

```text
python scripts/phase0_git_probe.py
```

El [script](../../scripts/phase0_git_probe.py) utiliza repositorios y configuraciones temporales. Ejecuta Git real y genera un commit sintético para verificar nombre y correo del autor y del committer. La configuración del desarrollador y los remotos quedan fuera de los fixtures. Cada ejecución imprime versión, arquitectura, casos y omisiones.

El [workflow de viabilidad](../../.github/workflows/phase0-viability.yml) repite las pruebas en Windows y Linux. Se puede ejecutar desde **Actions → Phase 0 Git viability** cuando el workflow esté en la rama predeterminada. [Ejecución manual en GitHub Actions](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)

## Resultados

| Plataforma | Git | Resultado |
| --- | --- | --- |
| Windows 11 x86_64 | `2.55.0.windows.5` | 12 aprobados. Junction verificada. Symlink omitido por falta de privilegios del usuario de prueba. |
| Linux x86_64, Debian Bookworm en contenedor sobre WSL2 | `2.39.5` | 12 aprobados. Symlink verificado. Caso de junction exclusivo de Windows omitido. |

| Caso observado | Decisión |
| --- | --- |
| HOME y XDG coexistentes. Un include posterior puede prevalecer. | Elegir el destino global que usaría Git, añadir una sola referencia administrada y comprobar precedencia con Git. Un entorno con redirección queda en diagnóstico. |
| Raíces solapadas, excepción exacta y commit en un repo creado después de declarar la regla. | Cargar la regla más específica después de la general y la excepción exacta al final. Git conserva el comportamiento fuera de la aplicación. |
| Configuración local por encima del include. Variables de autor separadas de `user.email`. | Mostrar identidad esperada, identidad configurada y autor/committer observado como resultados distintos. |
| `/**/.git` deja fuera metadatos habituales de worktrees vinculados, submódulos y bare. Un bare con metadatos terminados en `.git` aún puede coincidir. | Usar el patrón acotado para repositorios ordinarios. Diagnosticar topologías especiales y bloquear planes con efectos fuera de alcance ya detectados. |
| Rutas con espacios, Unicode, corchetes, diferencias de mayúsculas, symlink y junction. | Escapar literales antes de serializar, partir de rutas canónicas y usar coincidencia sensible por defecto. |
| Git rechaza escrituras ante un `.lock` existente. La edición posterior invalida la huella de la vista previa. | Usar exclusión por destino y huellas. Revalidar también antes de restaurar. |
| Sustitución local y restauración con archivos sintéticos. | Fija un caso base. Cifrado, permisos, durabilidad y fallos del motor se validarán en las pruebas de integración del adaptador. |

## Decisiones de cierre

- [M1–M6](../product/PRD.md) y los [wireframes](../ux/WIREFRAMES.md) definen la entrega funcional y el alcance de escritura por topología.
- Adoptar Git `2.39.5` como mínimo de producto. El fixture más antiguo ejecutado usa esa versión en Linux. En Windows se probó con Git `2.55.0.windows.5`. El onboarding comprobará también las capacidades del binario detectado.
- Los [ADR de Git](../decisions/ADR-002-git-nativo.md) y [recuperación](../decisions/ADR-003-recuperacion.md) registran estados y precondiciones. La validación nativa se incorpora al desarrollo del adaptador antes de habilitar escrituras reales.
