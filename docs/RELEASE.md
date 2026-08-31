# Releases

LoLTerm usa una versión global para workspace Rust, core y desktop package.

## Versionado

Formato SemVer:

```text
MAJOR.MINOR.PATCH
```

Durante `0.x`, la API todavía puede cambiar, pero se deben evitar roturas gratuitas.

## Release Please

Release Please lee Conventional Commits y mantiene:

- `CHANGELOG.md`;
- `Cargo.toml` (`workspace.package.version`);
- `apps/desktop/package.json` (`version`);
- `.release-please-manifest.json`.

Workflow: `.github/workflows/release-please.yml`.

## Packaging Linux

El workflow `.github/workflows/pack.yml` corre en tags `v*` y `workflow_dispatch`:

1. instala toolchains/dependencias;
2. ejecuta `pnpm install --frozen-lockfile` y `pnpm run pack` en `apps/desktop`;
3. genera `SHA256SUMS.txt`;
4. sube el `.deb`, el `.rpm`, el `.AppImage` y checksums como artifact;
5. si el evento es tag, adjunta los artefactos a la GitHub Release.

Artefactos esperados:

```text
LoLTerm-<version>-linux-<arch>.deb
LoLTerm-<version>-linux-<arch>.rpm
LoLTerm-<version>-linux-<arch>.AppImage
SHA256SUMS.txt
```

## Updater

El updater detecta el formato (`deb` vs `rpm` vs `appimage`), busca la última release estable de GitHub, elige el paquete compatible, descarga `SHA256SUMS.txt` y verifica SHA256 antes de instalar.

- `.deb` / `.rpm`: `pkexec` (`apt-get` / `dnf`) o el instalador del sistema.
- `.AppImage`: reemplaza el archivo en ejecución y reinicia la app (sin `pkexec`).

No hay firma GPG ni apt repo propio todavía. Si eso cambia, actualizar `SECURITY.md`, README y este documento en el mismo PR.

## Checklist manual antes de publicar

- CI verde.
- Release contiene `.deb`, `.rpm`, `.AppImage` y `SHA256SUMS.txt`.
- SHA256 coincide.
- `/update` detecta la release latest.
- GNOME muestra el icono `lolterm`, no el icono genérico de Electron.
- No hay `.env`, `.pem`, `id_rsa`, `session.toml`, swap files ni build outputs en el source archive.