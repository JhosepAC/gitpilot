# Distribución de GitPilot

## Plataformas

| Sistema | Instalador | Requisitos |
| --- | --- | --- |
| Windows x64 | `GitPilot_1.0.0_x64-setup.exe` | WebView2 Runtime y Git 2.39.5 o posterior. La instalación NSIS es por usuario. |
| Debian 12 amd64 | `GitPilot_1.0.0_amd64.deb` | Git 2.39.5 o posterior, WebKitGTK 4.1 y las dependencias declaradas por el paquete. Las funciones de credenciales y respaldos cifrados requieren Secret Service. |

Usa el NSIS sobre la instalación existente de Windows o ejecuta `sudo apt install ./GitPilot_1.0.0_amd64.deb` en Debian. Cierra GitPilot antes de actualizar. Al abrirlo, comprueba tus perfiles, directorios y preferencias.

Antes de actualizar, puedes exportar perfiles, asignaciones y preferencias desde **Configuración**. El archivo JSON contiene correos y rutas locales. Para recuperar una escritura Git, abre **Actividad** y revisa el destino de la operación. La restauración verifica el estado actual y protege las ediciones posteriores. [Portabilidad y almacenamiento](architecture/DATABASE.md#portabilidad) · [Recuperación de Git](architecture/GIT_CONFIGURATION.md)

## Preparar el candidato

1. Integra el cambio en `main` y espera a que pase **Desktop CI** para ese commit.
2. Ejecuta **Release candidate** desde `main`. El flujo exige CI aprobada para el mismo commit y compara las versiones de Tauri, npm y Cargo.
3. Descarga el artefacto `gitpilot-<versión>-candidate` de la ejecución. `BUILD-INFO.txt` registra el commit y el ID de la ejecución. El artefacto contiene NSIS, el paquete Debian, los avisos legales y sus sumas.
4. Verifica SHA-256 y prueba instalación limpia, actualización desde la versión anterior, reapertura con datos existentes y desinstalación. Usa cuentas y bases aisladas.

En Windows, compara el hash con la línea del instalador en `SHA256SUMS.txt`. En Debian, ejecuta la comprobación desde la carpeta de los artefactos.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath .\GitPilot_1.0.0_x64-setup.exe
```

```sh
sha256sum --check SHA256SUMS.txt
```

## Firmar y publicar

1. Para Windows, configura el servicio o certificado de firma del mantenedor y recompila el NSIS desde el commit verificado. Usa la configuración de firma de Tauri para firmar el ejecutable y el instalador durante el empaquetado. Comprueba ambos con `signtool verify /pa /all` y verifica la marca de tiempo. [Firma de Windows en Tauri](https://v2.tauri.app/distribute/sign/windows/)
2. Genera `SHA256SUMS.txt` a partir de los archivos finales y crea una firma OpenPGP separada del manifiesto. Distribuye la clave pública y su huella por un canal del proyecto que se pueda verificar. Conserva `LICENSE` y `NOTICE` con los instaladores.
3. Crea el tag firmado `v1.0.0` sobre el mismo commit y prepara un borrador en GitHub Releases con los archivos finales y las notas de [CHANGELOG.md](../CHANGELOG.md). Usa `gh release create v1.0.0 --verify-tag --draft` con los artefactos y notas revisados. [GitHub CLI](https://cli.github.com/manual/gh_release_create)
4. Descarga los archivos del borrador y vuelve a verificar firma, sumas, instalación y actualización. Publica el borrador cuando el responsable apruebe los resultados.

```sh
gpg --armor --detach-sign SHA256SUMS.txt
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
sha256sum --check SHA256SUMS.txt
```

Para verificar una publicación, coteja primero la huella OpenPGP con la anunciada por el proyecto. El NSIS también muestra el editor firmado en las propiedades del archivo.
