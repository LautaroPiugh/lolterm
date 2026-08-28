# Checklist para publicar el repositorio

Este checklist evita subir secretos, estado local o artefactos generados cuando el repositorio pase a público.

## Bloqueante

No publicar si aparece alguno:

- `.env` o `.env.*` con valores reales;
- claves privadas (`*.pem`, `id_rsa`, `id_ed25519`, `*.key`);
- tokens (`ghp_`, `github_pat_`, `sk-...`, `xox...`, etc.);
- `session.toml` con estado local;
- logs o diagnósticos con salida de terminal;
- paquetes `.deb`/AppImage generados fuera de GitHub Releases;
- swap files (`*.swp`, `.*.swp`).

## Comandos de revisión sugeridos

```bash
git status --short --ignored
git ls-files
```

Revisá que los archivos ignorados no formen parte de un zip/tar manual del proyecto.

## Archivos ignorados esperados

Pueden existir localmente, pero no deben subirse como código fuente:

- `target/`;
- `apps/desktop/dist/`;
- `apps/desktop/release/`;
- `apps/desktop/sidecar/`;
- `apps/desktop/build/icons/`;
- `apps/desktop/build/metainfo/`;
- `apps/desktop/node_modules/`.

## Documentación pública mínima

Antes de abrir el repo, deberían existir:

- `README.md`;
- `LICENSE`;
- `SECURITY.md`;
- `PRIVACY.md`;
- `CONTRIBUTING.md`;
- `docs/ARCHITECTURE.md`;
- `docs/DEVELOPMENT.md`;
- `docs/RELEASE.md`.

## Resultado del audit actual

En este árbol no se encontraron secretos reales ni archivos `.env`/claves privadas. Sí había swap files con metadata local; fueron eliminados. Quedan artefactos generados ignorados (`target/`, `apps/desktop/release/`, `apps/desktop/sidecar/`) que no afectan un push Git normal, pero sí deben excluirse si se comparte una carpeta comprimida.