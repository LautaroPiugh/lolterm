# Contribuir

Gracias por querer mejorar LoLTerm. El proyecto prioriza una terminal sólida antes que features grandes.

## Alcance del producto

LoLTerm organiza herramientas CLI existentes. No busca reemplazar nvim, Git, SSH, tmux ni agentes de IA. Antes de proponer una feature, preguntá si fortalece a LoLTerm como workspace de terminales o si reimplementa una herramienta madura.

## Stack

- Rust 2024 en `crates/core`.
- Electron main/preload en `apps/desktop/electron`.
- React + TypeScript + Vite en `apps/desktop/src`.
- xterm.js para emulación VT en el renderer.
- `portable-pty` para PTYs reales desde el core Rust.

## Desarrollo local

Requisitos:

- Rust stable con `rustfmt` y `clippy`;
- Node.js 22;
- npm.

Comandos principales:

```bash
# desde la raíz
cargo fmt --all -- --check
cargo clippy -p lolterm-core --all-targets -- -D warnings
cargo test --workspace

# desktop
cd apps/desktop
npm ci
npm run build
npm run dev
```

`npm run dev` levanta Vite y Electron para desarrollo. El paquete Linux se genera con:

```bash
cd apps/desktop
npm run pack
```

## Commits

Usá Conventional Commits:

```text
feat: add pane zoom
fix: restore terminal focus after tab switch
docs: document update trust model
refactor: simplify pty lifecycle
```

No crees commits automáticos desde agentes o scripts salvo que el mantenedor lo pida explícitamente.

## Pull requests

Un PR debería incluir:

- problema concreto;
- solución elegida;
- archivos tocados;
- pruebas realizadas;
- riesgos o limitaciones.

Para cambios de terminal, probá programas interactivos reales cuando aplique: `nvim`, `lazygit`, `btop`, `fzf`, `yazi`, `ssh`, `tmux`.

## Seguridad

No subas `.env`, claves SSH, tokens, dumps de sesión ni logs con salida de PTY. Leé `SECURITY.md` y `PRIVACY.md` antes de tocar HTTP LAN, updater, preload, contexto o agentes.