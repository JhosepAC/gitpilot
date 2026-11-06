# Resolución de identidad

## Tres resultados distintos

1. **Esperada por GitPilot.** Excepción explícita del repositorio, regla del directorio ancestro más específico, predeterminado habilitado dentro del ámbito administrado, o ausencia de expectativa. Elegir por componentes de ruta canónica. Si hay un empate tras normalizar, rechazar la regla duplicada y pedir corrección.
2. **Configurada en Git.** Valor de cada clave con origen y ámbito observados por Git. Conservar en el diagnóstico configuraciones local o worktree explícitas y comparar con la expectativa. Si falta una regla, presentar la identidad heredada o local existente como resultado de Git. Si faltan datos, mostrar advertencia.
3. **Autor y committer observados.** Consultar `git var GIT_AUTHOR_IDENT` y `git var GIT_COMMITTER_IDENT` para el contexto de verificación. El proceso de un editor o terminal puede tener otros valores de entorno o argumentos. Mostrar este límite cuando proceda. [Documentación de git var](https://git-scm.com/docs/git-var)

La expectativa contiene el par nombre/correo. Comparar ambos campos con sus respectivos orígenes. El resultado incluye regla y prioridad, configuración por campo, condición del repositorio y acciones válidas. Mantener por separado diagnóstico de firma y acceso remoto.

| Caso | Resultado esperado |
| --- | --- |
| Raíz `Proyectos` y subdirectorio `Proyectos/Empresa` | Gana Empresa en sus descendientes. `Empresa-Archivo` queda fuera de esa regla específica. |
| Excepción Manual dentro de Empresa | La expectativa es Manual. La lectura de Git muestra si realmente se aplica. |
| Git tiene otro correo en el ámbito local | Mostrar discrepancia y origen. Conservar o preparar un plan explícito para aplicar la expectativa. |
| Raíz registrada sin regla y predeterminado habilitado | Ofrecer el predeterminado para esa raíz. Previsualizar su alcance antes de materializarlo. |
| Ruta sin expectativa administrada | Mostrar la configuración que Git encuentre, incluso si falta identidad. |
| Worktree, submódulo o `.git` separado | Evaluar por ruta visible y contrastar con el Git dir real. Mostrar divergencias y límites de aplicación. |

Una decisión de conservar mantiene Git tal como está y registra el repositorio, revisión de política y huella del conflicto. Si la regla o el estado observado cambian, reevaluar. Si el valor procede de una fuente que el plan no puede modificar de forma segura, explicar su origen y mantener diagnóstico de solo lectura.

Las exclusiones del MVP limitan el descubrimiento. Una exclusión bajo una regla de carpeta puede seguir heredando configuración nativa. La vista previa debe indicarlo. Para limitar la aplicación, el usuario puede registrar raíces más estrechas. Resolver exclusiones nativas por descendiente queda pendiente de viabilidad en Git.
