# Distribución de GitPilot

## Plataformas

| Sistema | Instalador | Requisitos |
| --- | --- | --- |
| Windows x64 | `GitPilot_1.1.0_x64-setup.exe` | WebView2 Runtime y Git 2.39.5 o posterior. La instalación NSIS es por usuario. |
| Debian 12 amd64 | `GitPilot_1.1.0_amd64.deb` | Git 2.39.5 o posterior, WebKitGTK 4.1 y las dependencias declaradas por el paquete. Las funciones de credenciales y respaldos cifrados requieren Secret Service. |

Usa el NSIS sobre la instalación existente de Windows o ejecuta `sudo apt install ./GitPilot_1.1.0_amd64.deb` en Debian. Cierra GitPilot antes de actualizar. Al abrirlo, comprueba tus perfiles, directorios y preferencias.

Antes de actualizar, puedes exportar perfiles, asignaciones y preferencias desde **Configuración**. El archivo JSON contiene correos y rutas locales. Para recuperar una escritura Git, abre **Actividad** y revisa el destino de la operación. La restauración verifica el estado actual y protege las ediciones posteriores. [Portabilidad y almacenamiento](architecture/DATABASE.md#portabilidad) · [Recuperación de Git](architecture/GIT_CONFIGURATION.md)

## Preparar el candidato

1. Integra el cambio en `main` y espera a que pase **Desktop CI** para ese commit.
2. Ejecuta **Release candidate** desde `main`. El flujo exige CI aprobada para el mismo commit y compara las versiones de Tauri, npm y Cargo.
3. Descarga el artefacto `gitpilot-<versión>-candidate` de la ejecución. `BUILD-INFO.txt` registra el commit y el ID de la ejecución. El artefacto contiene NSIS, el paquete Debian, los avisos legales y sus sumas.
4. Verifica SHA-256 y prueba instalación limpia, actualización desde la versión anterior, reapertura con datos existentes y desinstalación. Usa cuentas y bases aisladas.

En Windows, compara el hash con la línea del instalador en `SHA256SUMS.txt`. En Debian, ejecuta la comprobación desde la carpeta de los artefactos.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath .\GitPilot_1.1.0_x64-setup.exe
```

```sh
sha256sum --check SHA256SUMS.txt
```

## Firmar y publicar

El NSIS de v1.1.0 se distribuye sin Authenticode. La firma OpenPGP del manifiesto permite verificar la procedencia y la integridad de ambos paquetes antes de instalar. Windows puede mostrar el instalador con editor desconocido. [Firma de Windows en Tauri](https://v2.tauri.app/distribute/sign/windows/)

1. Usa la clave OpenPGP de distribución del mantenedor. Si vas a crearla con GnuPG, elige una frase de paso en el diálogo local y guarda el material de recuperación en privado. Comparte solo la clave pública y la huella completa. [Gestión de claves en GnuPG](https://gnupg.org/documentation/manuals/gnupg/OpenPGP-Key-Management.html)
2. Desde la carpeta de los artefactos definitivos, verifica las sumas y firma `SHA256SUMS.txt` con la huella de distribución. Exporta la clave pública como `GitPilot-release-key.asc`. Conserva `LICENSE` y `NOTICE` con los instaladores.
3. Prepara un borrador de GitHub Releases dirigido al SHA verificado con `gh release create v1.1.0 --target "$SHA" --draft`. Adjunta los archivos y las notas de [CHANGELOG.md](../CHANGELOG.md). Si la firma se completa después, añádela al mismo borrador. [GitHub CLI](https://cli.github.com/manual/gh_release_create)
4. Descarga los archivos del borrador y verifica firma, sumas, instalación y actualización. Tras la aprobación del responsable, crea y sube el tag firmado `v1.1.0` sobre el mismo commit, comprueba su destino y publica el borrador.

Con `FINGERPRINT` configurada con la huella completa de la clave de distribución, ejecuta:

```sh
sha256sum --check SHA256SUMS.txt
gpg --local-user "$FINGERPRINT" --armor --detach-sign SHA256SUMS.txt
gpg --armor --output GitPilot-release-key.asc --export "$FINGERPRINT"
```

Para verificar una publicación, coteja primero la huella de la clave con la anunciada por el mantenedor a través de un canal de confianza. Luego comprueba la firma y las sumas.

```sh
gpg --show-keys --fingerprint GitPilot-release-key.asc
gpg --import GitPilot-release-key.asc
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
sha256sum --check SHA256SUMS.txt
```

## Candidato 1.1.0

La publicación debe usar el SHA final de `main` aprobado por Desktop CI. Las notas de versión están en `docs/releases/v1.1.0.md`. El NSIS local es un candidato de Windows; no sustituye la compilación de ambas plataformas en Release candidate ni acredita las pruebas de actualización.

La actualización agrega referencias de autenticación y firma y conserva la obligatoriedad de firma para perfiles anteriores que ya tenían clave. Revisa las rutas de las claves y reaplica cada perfil desde Directorios para materializar su política. Un perfil sin clave propia bloquea la autenticación SSH por clave. HTTPS sigue usando su helper configurado.

Las correcciones y el alcance de validación autorizado para 1.1.0 se detallan en [la revisión de seguridad](security/REVIEW_1_1_0.md). Las comprobaciones adicionales Linux se omiten por decisión del responsable. Incluir THIRD_PARTY_NOTICES.txt con los artefactos y en el manifiesto firmado.
