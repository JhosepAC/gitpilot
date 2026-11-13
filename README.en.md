# GitPilot

[Español](README.md)

GitPilot is a local-first desktop app for managing Git identities on Windows and Linux x86_64. macOS is planned for a future release.

- **Identities** stores author profiles and lets you review an import from your existing Git configuration. **Directories** scans working folders, supports exclusions and repository overrides, and compares your expected identity with Git's effective settings.
- **Git preview** shows the files and rules affected by a change. After confirmation, GitPilot backs up, applies, and verifies the configuration. Use **Activity** to review an operation or restore an earlier configuration.
- **Inspector** shows branch, index, and remote information. It offers separate checks for SSH, HTTPS, and commit signing, plus an optional pre-commit identity check.
- **GitHub** connects accounts through Device Flow and shows their authorized installations, repositories, pull requests, and issues. You can link a local repository to a matching GitHub remote.
- **Settings** exports versioned JSON and previews imports before applying them to GitPilot's local database. The language preference covers navigation, Home, Settings, and the Activity list. Detailed forms and diagnostics use Spanish.

## Install and verify

The [official 1.1.0 release](https://github.com/JhosepAC/gitpilot/releases/tag/v1.1.0) provides [Windows x64](https://github.com/JhosepAC/gitpilot/releases/download/v1.1.0/GitPilot_1.1.0_x64-setup.exe) and [Debian 12 amd64](https://github.com/JhosepAC/gitpilot/releases/download/v1.1.0/GitPilot_1.1.0_amd64.deb). Verify the OpenPGP manifest and checksums before installing. The Windows installer does not include Authenticode signing.

GitPilot supports Windows x64 with WebView2 and Linux amd64 with WebKitGTK 4.1. Use Git 2.39.5 or later. The Debian package is built on Debian 12 and declares Git and `libsecret-tools` as dependencies. Credential and encrypted backup features on Linux need an available Secret Service. [Installation, verification, and recovery](docs/RELEASING.md)

## Develop

Install Node.js 24, npm, and Rust through rustup. Windows needs Visual Studio C++ Build Tools, the Windows SDK, and WebView2. Debian or Ubuntu needs the desktop libraries listed in the [Spanish README](README.md#ejecutar-en-desarrollo).

```sh
cd apps/desktop
npm ci
npm run tauri -- dev
```

Use `npm run tauri -- build --ci -- --locked` to build the NSIS installer on Windows or the Debian package on Linux. The [contribution guide](CONTRIBUTING.md) lists the validation commands.

## License

GitPilot was created by Jhosep Jamil Argomedo Camacho. Its source is available for noncommercial purposes under CC BY-NC 4.0. Retain the attribution and license and indicate any modifications when sharing copies or adaptations. For internal commercial use, sales, or monetization, request permission from the author. [License](LICENSE) · [Attribution](NOTICE) · [@JhosepAC](https://github.com/JhosepAC)

[Contributing](CONTRIBUTING.md) · [Private vulnerability reports](SECURITY.md) · [Changelog](CHANGELOG.md)
