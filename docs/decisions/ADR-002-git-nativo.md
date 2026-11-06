# ADR-002 · Git como autoridad

**Estado** · Adoptado para el MVP en Windows y Linux.

Usar Git CLI para observar configuraciones y autor/committer. Usar una inclusión global administrada con condiciones `gitdir:<raíz>/**/.git` para repositorios ordinarios y Git dir exacto para excepciones. Seleccionar HOME o XDG según el archivo efectivo que usaría Git para escribir globalmente y degradar a diagnóstico ante redirecciones o ámbitos ambiguos. El resultado de Git tras la escritura determina si el plan tuvo éxito.

Git `2.39.5` es el mínimo del MVP. Esa versión se verificó en Linux. La ejecución de Windows utilizó Git `2.55.0.windows.5`. Comprobar versión y capacidades en onboarding. La compatibilidad depende del binario y el contexto que usen terminal y editor. `includeIf.worktree` queda fuera del contrato de compatibilidad. [Configuración](../architecture/GIT_CONFIGURATION.md) · [Evidencia](../testing/PHASE_0.md)
