# Pruebas y validación

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
| `release.yml` | Tags versionados de `main` con CI verificada. Construir, firmar donde proceda, generar checksums, comprobar instalación y publicar mediante permisos acotados y secretos del entorno de release. |

La definición de terminado exige criterios de aceptación satisfechos, validaciones ejecutadas y registradas, cobertura de todos los casos críticos aplicables y documentación actualizada. Toda escritura Git requiere pruebas de integración. Los casos de cada plataforma se registran con versión de Git, sistema, resultados y límites. La publicación exige resultados favorables en ambas plataformas objetivo.

## Evidencia de fase 1

| Plataforma | Validación ejecutada |
| --- | --- |
| Windows 11 x86_64 | Compilación Tauri y arranque con WebView2. El texto de la ventana confirmó la respuesta IPC y se comprobó la migración aplicada en SQLite. |
| Debian Bookworm x86_64 sobre WSL2 | Compilación Tauri y arranque con WebKitGTK en una pantalla virtual. Se comprobó el estado visible de almacenamiento y la migración en un directorio temporal tanto con el binario compilado como con `npm run tauri -- dev`. |

Prettier, ESLint, typecheck, cuatro pruebas de Vitest, formato de Rust, Clippy y dos pruebas de almacenamiento Rust pasaron en ambos entornos. La [CI](../../.github/workflows/ci.yml) reproduce las comprobaciones de compilación y pruebas cuando los cambios se publiquen en GitHub.
