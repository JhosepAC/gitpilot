# GitPilot

GitPilot es una aplicación de escritorio local-first para gestionar identidades Git. El lanzamiento apunta a Windows y Linux x86_64. macOS queda como ampliación futura.

La sección **Identidades** permite crear, editar, duplicar y eliminar perfiles locales, y elegir un predeterminado. **Importar desde Git** lee el nombre y el correo globales y abre un borrador para revisión antes de guardar.

En **Directorios**, añade la ruta absoluta de una carpeta de trabajo y pulsa **Escanear**. Puedes excluir subcarpetas, asignar un perfil al directorio y definir excepciones por repositorio. Abre un repositorio para comparar la identidad esperada con la configuración que Git devuelve. GitPilot guarda las reglas en SQLite y consulta Git en modo de solo lectura. La aplicación a Git corresponde a la fase 4. [Roadmap](docs/ROADMAP.md)

Pulsa **Vista previa de Git** para ver qué archivo global recibiría la inclusión, qué reglas se generarían para todo el inventario y qué valores prevalecerían en los repositorios que GitPilot puede verificar. La simulación usa archivos temporales y conserva la configuración existente.

Cuando la vista previa se puede verificar, elige **Revisar aplicación** y confirma el destino. GitPilot guarda un respaldo cifrado de la configuración global existente, activa las reglas y verifica el resultado con Git. Abre de nuevo la vista previa para revisar la restauración. Para aplicar otro conjunto de reglas, restaura primero la aplicación anterior. En Linux, respaldar un archivo existente requiere `secret-tool` y un servicio Secret Service disponible en la sesión de escritorio.

**Inicio** comprueba Git y guía la creación o importación de perfiles y el registro de carpetas. En el diagnóstico de un repositorio, puedes registrar que quieres conservar el valor actual o revisar un cambio local compatible. **Actividad** muestra esas decisiones y el estado de las escrituras. Desde allí puedes revisar una restauración global o local.

## Ejecutar en desarrollo

Instala Node.js 24, npm y Rust mediante rustup. El archivo `rust-toolchain.toml` selecciona Rust y sus componentes de formato y lint.

- **Windows.** Visual Studio Build Tools con las herramientas de C++ y Windows SDK, WebView2 Runtime y PowerShell. Si rustup no agregó Cargo a tu PATH, ejecuta `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` en la sesión actual.
- **Linux.** En Debian o Ubuntu, instala las dependencias de escritorio y compilación que aparecen abajo. Para otras distribuciones, consulta los paquetes equivalentes. [Requisitos de Tauri](https://v2.tauri.app/start/prerequisites/)

```sh
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Desde la raíz del repositorio:

```text
cd apps/desktop
npm ci
npm run tauri -- dev
```

La pantalla **Identidades** se abre después de aplicar las migraciones y comprobar el almacenamiento. SQLite se guarda en el directorio local de datos de la aplicación, resuelto por Tauri. Los tests de Rust usan una ubicación temporal. Git `2.39.5` es el mínimo para las funciones Git del MVP. [Evidencia de compatibilidad](docs/testing/PHASE_0.md)

## Validar y compilar

Desde `apps/desktop`:

```text
npm run format:check
npm run lint
npm run typecheck
npm test
npm run tauri -- build --debug --no-bundle --ci -- --locked
cd src-tauri
cargo fmt --all -- --check
cargo clippy --locked --features custom-protocol --all-targets -- -D warnings
cargo test --locked --features custom-protocol
```

La compilación anterior genera un binario de desarrollo. Las reglas de contribución usan [trunk-based development](docs/ROADMAP.md#trunk-based-development). Los contextos locales de asistentes se excluyen del historial y CI comprueba que esas rutas no estén rastreadas.

## Generar los instaladores de v0.1.0

Desde `apps/desktop`, ejecuta `npm run tauri -- build --ci -- --locked` en cada plataforma. Windows genera `GitPilot_0.1.0_x64-setup.exe`, un instalador NSIS por usuario. Linux genera `GitPilot_0.1.0_amd64.deb` con Debian 12 como base de compilación. El paquete declara Git y `libsecret-tools` como dependencias. En Debian, instálalo con `sudo apt install ./GitPilot_0.1.0_amd64.deb` desde la carpeta que contiene el archivo. Los candidatos locales incluyen un archivo de sumas SHA-256. La firma de distribución y la publicación quedan pendientes. [Empaquetado Debian de Tauri](https://v2.tauri.app/distribute/debian/) · [Instaladores de Windows](https://v2.tauri.app/distribute/windows/)
