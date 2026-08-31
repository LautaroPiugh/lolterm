# Desarrollo

## Requisitos

- Linux para el flujo principal actual.
- Rust stable con componentes `rustfmt` y `clippy`.
- Node.js 22.
- pnpm.

## Instalar dependencias

```bash
cd apps/desktop
pnpm install --frozen-lockfile
```

Rust usa el workspace de la raíz y resuelve dependencias con Cargo.

## Ejecutar en desarrollo

```bash
cd apps/desktop
pnpm run dev
```

Esto levanta Vite para el renderer y abre Electron. En desarrollo puede usarse `--no-sandbox` porque el binario local de Electron no siempre tiene el helper SUID de Chromium configurado. El paquete `.deb` usa otra ruta de sandbox.

## Build desktop

```bash
cd apps/desktop
pnpm run build
```

Genera `apps/desktop/dist/`, ignorado por Git.

## Pack Linux `.deb`, `.rpm` y `.AppImage`

```bash
cd apps/desktop
pnpm run pack
```

El script compila el sidecar Rust, sincroniza iconos/metainfo y crea artefactos en `apps/desktop/release/`. Esa carpeta está ignorada por Git.

## Checks de Rust

```bash
cargo fmt --all -- --check
cargo clippy -p lolterm-core --all-targets -- -D warnings
cargo test --workspace
```

## Checks de desktop

```bash
cd apps/desktop
pnpm install --frozen-lockfile
pnpm run build
```

## Archivos generados ignorados

No subir:

- `target/`;
- `apps/desktop/dist/`;
- `apps/desktop/release/`;
- `apps/desktop/sidecar/`;
- `apps/desktop/build/icons/`;
- `apps/desktop/build/metainfo/`;
- `node_modules/`;
- swap files (`*.swp`, `.*.swp`).

## Variables útiles

| Variable | Uso |
| --- | --- |
| `LOLTERM_CORE` | override del binario core en desarrollo |
| `LOLTERM_DEV` | marca ejecución dev |
| `LOLTERM_URL` | URL del renderer Vite en dev |
| `GITHUB_TOKEN` / `GH_TOKEN` | opcional para evitar rate limit al probar GitHub Releases privadas o con límite bajo |

No documentes ni comitees valores reales de esas variables.