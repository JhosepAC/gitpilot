# GitPilot · requisitos de producto

## Problema y objetivo

Quien alterna entre proyectos personales, corporativos y de clientes puede crear commits con un nombre o correo inesperado. GitPilot centraliza perfiles y asignaciones, explica de dónde sale la identidad y permite aplicar y restaurar cambios de configuración. Será una aplicación de escritorio open source, local-first, para Windows y Linux. macOS queda como ampliación futura.

| Actor | Necesidad y caso de uso |
| --- | --- |
| Desarrollador con varias identidades | Registrar perfiles, aplicar reglas por carpeta y crear commits con la identidad esperada al cambiar de proyecto. |
| Desarrollador con una excepción | Asignar otro perfil a un repositorio, entender una diferencia con Git y decidir si conserva o cambia la configuración. |
| Desarrollador con varias cuentas remotas | Diagnosticar acceso SSH/HTTPS, asociar cuentas y consultar recursos autorizados de GitHub. |
| Mantenedor o colaborador | Reproducir escenarios con datos de prueba, validar compatibilidad y publicar versiones desde `main`. |

## Inventario funcional

| ID | Módulo | MVP v0.1.0 | Evolución hasta v1.0 |
| --- | --- | --- | --- |
| F1 | Identity Manager | Crear, consultar, editar, duplicar y eliminar perfiles. Validar nombre y correo, mostrar usos y permitir un predeterminado optativo. Importar de forma explícita la identidad detectada. | Usuario GitHub opcional, referencias a firma OpenPGP/SSH y preferencia opcional de rama para operaciones futuras de inicialización. |
| F2 | Repository & Directory Manager | Registrar y excluir directorios, descubrir repositorios anidados, ver ruta y estado, asignar identidad por carpeta y por excepción de repositorio. | Ampliar compatibilidad de topologías tras pruebas y mejorar filtros y descubrimiento incremental. |
| F3 | Identity Resolver y configuración Git | Resolver reglas, explicar precedencia, inspeccionar origen y ámbito, detectar discrepancias, previsualizar, aplicar, verificar y restaurar. | Ampliar diagnóstico y variantes de materialización con evidencia de compatibilidad. |
| F4 | SSH & Authentication Manager | — | Referenciar claves existentes, asociar perfiles de autenticación, diagnosticar permisos, alias, hosts, identidad remota y configuración HTTPS con el helper existente. |
| F5 | Git Inspector | Identidad esperada, valores Git efectivos, origen, ámbito y discrepancias. | Rama, estado del árbol, remotos, firma, cuenta SSH comprobable y pre-commit optativo. |
| F6 | GitHub Integration | — | OAuth multicuenta, desconexión y revocación, perfil, organizaciones, repositorios autorizados, vínculo con remotos, PR e issues. |
| F7 | Settings | Tema claro/oscuro/sistema, exclusiones, preferencias de escaneo y gestión de respaldos. | Idioma, notificaciones, importación/exportación y preferencias de inicio. |
| F8 | Activity & Audit | Historial local de aplicar, conservar, restaurar y fallar. | Eventos de perfiles, reglas, descubrimiento, conexión y diagnósticos, con filtros y retención. |

Los datos permanecen en el dispositivo. El MVP funciona sin red y el escaneo se activa al registrar una carpeta o por solicitud del usuario. Se limita a raíces autorizadas y ofrece progreso, cancelación y resultados parciales. La instalación y el onboarding inspeccionan Git y muestran propuestas. La escritura externa empieza tras la confirmación del cambio concreto.

## Criterios de aceptación del MVP

| ID | Prueba de aceptación |
| --- | --- |
| M1 | Instalar y abrir en Windows y Linux. Detectar la versión de Git o explicar cómo habilitar un binario compatible. Crear tres perfiles y comprobar que sobreviven a un reinicio. |
| M2 | Registrar dos carpetas, personal y corporativa. Detectar repositorios, incluidos los anidados admitidos, y asociar una identidad a cada carpeta. Ver un repositorio sin asignación y excluir una ruta sin recorrerla. |
| M3 | Asignar el tercer perfil a un repositorio como excepción. Ver fuente, prioridad, destino, alcance de los cambios y confirmación antes de escribir. |
| M4 | Cerrar GitPilot. En una terminal externa consultar `git config user.email` y crear commits de prueba en los casos de carpeta y excepción. Verificar nombre y correo de autor y committer en el historial. Crear otro repositorio ordinario bajo una carpeta administrada y comprobar que hereda su regla sin volver a abrir la app. Comprobar un commit desde un editor en un entorno Git compatible. |
| M5 | Introducir una configuración local contradictoria. Mostrar valor, origen, ámbito y regla esperada. Permitir conservar el valor y registrar esa decisión, o aplicar la identidad mediante una confirmación explícita cuando el cambio sea compatible. |
| M6 | Restaurar el estado previo y verificarlo con Git. Simular una interrupción o fallo de escritura y comprobar recuperación. Ante edición externa posterior, detener la restauración automática y mostrar los pasos disponibles. |

## Requisitos de calidad

- Separación entre autor/committer, firma y autenticación remota. Las credenciales externas se conservan cuando se elimina un perfil.
- Cambios explicables y recuperables. La interfaz muestra los estados de escaneo, conflicto, plan pendiente, verificación y recuperación.
- Mínimo privilegio en procesos, filesystem, IPC y redes. Aplicar [controles de seguridad](../security/SECURITY.md) desde infraestructura.
- Navegación por teclado, legibilidad en ambos sistemas y redimensionamiento, según la [especificación de UX](../ux/UX.md). Medir escaneo y diagnóstico según [pruebas](../testing/TESTING.md).
