# GitPilot

GitPilot es una aplicación de escritorio local-first para gestionar identidades Git. El lanzamiento apunta a Windows y Linux x86_64. macOS queda como ampliación futura.

La sección **Identidades** permite crear, editar, duplicar y eliminar perfiles locales, y elegir un predeterminado. **Importar desde Git** lee el nombre y el correo globales y abre un borrador para revisión antes de guardar.

En **Directorios**, añade la ruta absoluta de una carpeta de trabajo y pulsa **Escanear**. Puedes excluir subcarpetas, asignar un perfil al directorio y definir excepciones por repositorio. Abre un repositorio para comparar la identidad esperada con la configuración que Git devuelve. GitPilot guarda las reglas en SQLite y consulta Git en modo de solo lectura. La aplicación a Git corresponde a la fase 4. [Roadmap](docs/ROADMAP.md)

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

La compilación genera un binario de desarrollo. La distribución de instaladores corresponde a hitos posteriores. Las reglas de contribución usan [trunk-based development](docs/ROADMAP.md#trunk-based-development). Los contextos locales de asistentes se excluyen del historial y CI comprueba que esas rutas no estén rastreadas.
