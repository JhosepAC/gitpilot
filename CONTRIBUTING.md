# Contribuir a GitPilot

Abre un issue para cambios de alcance o de comportamiento que necesiten discusión. Para reportar una vulnerabilidad, usa el [canal privado](SECURITY.md).

1. Parte de `main` y crea una rama corta, por ejemplo `feat/settings-export` o `fix/inventory-focus`.
2. Divide el trabajo en Conventional Commits. Incluye las pruebas y la documentación que correspondan al cambio.
3. Abre un PR a `main` con el resultado esperado, los casos verificados y las plataformas probadas. Tras la revisión y CI, integra en el tronco y retira la rama.

Usa bases, repositorios y configuraciones Git temporales en las pruebas. Mantén fuera de Git instrucciones de asistentes, skills, índices, credenciales y artefactos locales. La identidad de autor de Git, la firma y la autenticación son capacidades separadas. Toda escritura Git requiere vista previa, confirmación, respaldo, verificación y recuperación.

Ejecuta las comprobaciones desde `apps/desktop`.

```sh
npm ci
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

Consulta el [alcance de plataforma](README.md#ejecutar-en-desarrollo), el [roadmap](docs/ROADMAP.md) y la [licencia](LICENSE).

## English

Create short-lived `feat/` or `fix/` branches from `main`. Use Conventional Commits and open a PR to `main` with the expected behavior, checks, and platforms tested. Run the commands above with temporary repositories, databases, and Git configuration. Report vulnerabilities through the [private channel](SECURITY.md).
