# Seguridad y recuperación

## Activos y fronteras

Proteger configuraciones Git y SSH del usuario, repositorios, preferencias, respaldos y credenciales. Tratar rutas, archivos `.git`, valores Git, URLs, respuestas remotas y mensajes IPC como entradas no confiables. Rust es la frontera de privilegios. La interfaz opera sobre contenido local y datos serializados, con una CSP estricta y capacidades Tauri limitadas por ventana y comando. Abrir autenticación y enlaces remotos en el navegador del sistema. [Capacidades de Tauri](https://v2.tauri.app/security/capabilities/)

| Riesgo | Control y evidencia |
| --- | --- |
| Inyección de comandos o argumentos | Ejecutable Git validado, argumentos separados, claves y operaciones permitidas, separador de opciones cuando aplique, entorno filtrado y rutas verificadas. Sin shell ni ejecución arbitraria por IPC. Pruebas con metacaracteres y rutas maliciosas. |
| Repositorio o configuración maliciosa | Usar consultas acotadas, controlar subprocesos y helpers por operación, aplicar timeout y límite de salida. Evitar ejecutar hooks, helpers, transportes o comandos derivados de configuración durante inspección. Probar que un fixture hostil no ejecuta programas. |
| Escape de raíz, links o sustitución de destino | Resolución canónica y comprobación de identidad por plataforma antes y durante escritura. Escaneo sin seguir enlaces ni junctions y escritura conservadora ante destinos ambiguos. Autorización para los archivos de configuración que estén fuera de raíces. |
| Corrupción, concurrencia o pérdida de cambios | Usar vista previa, consentimiento, respaldo, locks, journal, verificación con Git y restauración condicionada. Pruebas de fallo por etapa y edición externa. |
| Exposición en DB, IPC, logs, copias o exportaciones | Secretos fuera de SQLite, WebView, logs, informes y exportaciones. Redactar URLs, valores, stderr y datos de trazas. Restringir permisos de app y retención. Pruebas con secretos señuelo. |
| Exposición en un respaldo de archivos externos | Tratar snapshots como sensibles incluso si solo se cambia `user.email`. Cifrar los respaldos de archivos externos con una clave protegida por el SO, restringir permisos y evitar copias intermedias en texto plano. Si no se puede crear y comprobar el respaldo protegido, detener la escritura. Verificar restauración byte a byte y permisos. |
| Acceso remoto excesivo o robo de tokens | Capacidades mínimas y almacenes seguros del SO. Mantener OAuth separado de SSH/HTTPS. Probar revocación, redirecciones, permisos insuficientes y almacén indisponible. |
| Alteración de dependencias o instaladores | Lockfiles, permisos mínimos en Actions, auditorías y revisión de dependencias. Firma, integridad y procedencia verificables en la publicación. |

Escanear y diagnosticar Git sin cambiar preferencias, llaves ni archivos externos. Cualquier cambio de configuración Git, SSH, firma o hooks sigue el [protocolo de escritura](../architecture/GIT_CONFIGURATION.md). Las operaciones destructivas de Git quedan fuera del flujo del producto. Usar auditoría local de acciones y resultados con referencias mínimas, sin volcar el entorno ni el contenido de los archivos.

Antes de publicar, documentar soporte, reporte privado de vulnerabilidades, tratamiento de datos, ubicación de respaldos, retención y recuperación. Definir la política de limpieza de generaciones antiguas de config, conservando las referenciadas por operaciones activas y restauraciones disponibles.
