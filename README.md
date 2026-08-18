# LoLTerm

LoLTerm es un **workspace de terminales local-first**. La unidad de trabajo no es el archivo: es el proceso que corre dentro de un PTY (bash, nvim, lazygit, ssh, un agente CLI, lo que sea).

> LoLTerm es el entorno desde el que trabajo, no la herramienta con la que hago cada trabajo.

No reemplaza nvim, Git, SSH, tmux ni Codex/Claude. Los **abre y organiza**.

```text
LoLTerm (ventana Electron)
    └── lolterm-core (sidecar Rust)
            ├── PTY local  → shell / nvim / lazygit / …
            └── PTY        → ssh → tmux en otra máquina
```

Versión actual: **0.9.x** (estabilización y distribución Linux). Empaquetado: **`.deb` para Ubuntu**.

## Qué no es

No es un IDE, un editor propio, un fork de tmux, un chat de IA ni un cliente Git. El runtime es siempre: proceso + PTY + emulador VT (xterm.js).

## Cómo funciona

1. Electron muestra la UI (tabs, splits, paleta, explorer).
2. `lolterm-core` crea pseudoterminales reales (`portable-pty`) y multiplexa panes.
3. Lo que escribís llega al proceso hijo; lo que el proceso imprime vuelve a xterm.js.
4. La CLI `lolterm` habla con el mismo core: si el Desktop está abierto, pregunta por un Unix socket de **solo lectura**; si no, lee `session.toml`.

```text
teclado  →  React / xterm.js  →  preload  →  Electron  →  JSON  →  lolterm-core  →  PTY
```

## Instalar (Ubuntu)

Cuando el repo es público, cada tag `v*` publica un `.deb` y `SHA256SUMS.txt` en [Releases](https://github.com/LautaroPiugh/lolterm/releases).

```bash
sudo apt install ./LoLTerm-*-linux-amd64.deb
```

El menú de aplicaciones usa el icono del prompt `>` sobre panes mint. `/update` (o el banner) busca una release más nueva, verifica el SHA256 y recién ahí instala (`pkexec` o el instalador del sistema).

## Desarrollo

```bash
# core
cargo test -p lolterm-core
cargo clippy -p lolterm-core --all-targets -- -D warnings

# desktop
cd apps/desktop
npm install
npm run dev
```

`npm run dev` compila `lolterm-core`, levanta Vite y abre Electron. Empaquetar:

```bash
cd apps/desktop && npm run pack
# sale apps/desktop/release/LoLTerm-*-linux-*.deb
```

## Uso

| Acción | Atajo |
| --- | --- |
| Paleta de comandos | `Ctrl-b` / `Ctrl-p` |
| Siguiente / anterior tab | `Ctrl-Tab` / `Ctrl-Shift-Tab` |
| Split derecha / abajo | `Ctrl-Alt-v` / `Ctrl-Alt-s` |
| Workspaces | clic en el nombre, o `Ctrl-Alt-[` `]` |
| Restart del pane | `Ctrl-Alt-r` |
| Comandos y atajos | `Ctrl-Alt-,` o `/commands` |
| Buscar update | `/update` |

`+` abre el picker (shell, SSH, agentes). Cerrar todas las tabs muestra Inicio. Arrastrar una tab al borde parte el layout.

## CLI

```bash
cargo install --path crates/core --bin lolterm --force --root "$HOME/.local"

lolterm status
lolterm context
lolterm panes
lolterm processes
lolterm machines
lolterm workspace list
lolterm .
lolterm workspace open lolterm
lolterm ssh home
lolterm run nvim
```

Config portable: `~/.config/lolterm/`. Estado de máquina: socket en `$XDG_RUNTIME_DIR/lolterm/` (no sincronizar). Cada PTY recibe `LOLTERM_ROOT` y `LOLTERM_CONTEXT`. Los agentes CLI pueden abrir en un `git worktree` bajo `~/.local/share/lolterm/worktrees/` (`[ai] worktrees = false` para desactivar).

## Remoto

```toml
# ~/.config/lolterm/config.toml
[remote]
user = "dev"
tmux = "lolterm"

[[machines]]
name = "home"
target = "home.example.ts.net"
user = "dev"
kind = "tailscale"
```

Flujo: `PTY → ssh -tt dest → tmux new-session -A -s lolterm-<workspace>`. LoLTerm agrega keepalives SSH; si el enlace se cae, reconecta **una vez**. La password la pide `ssh` en el pane, no LoLTerm.

## Extensiones

No hay plugins JS. Solo TOML local (`commands.toml`, `hooks.toml`, `themes/*.toml`, `status.toml`, `context.toml`, packs `extensions/<nombre>/extension.toml`). `run` tiene que ser un binario del PATH, sin flags ni paths.

## Repositorio

```text
lolterm/
├── README.md
├── AGENTS.md          # visión y reglas de arquitectura
├── Cargo.toml         # workspace Rust (lolterm-core)
├── crates/core/       # PTY, mux, SSH, CLI, sesión
└── apps/desktop/      # Electron + React + xterm.js
```

Las carpetas de herramientas de IA (`.cursor`, `.claude`, …) no forman parte del proyecto y están en `.gitignore`.

## Seguridad

- App **local-first**: no hay backend ni cuenta.
- El renderer no tiene Node (`contextIsolation`, preload chico).
- El socket del mux es `0600` y solo admite `context` / `panes` / `processes`.
- El auto-update solo descarga `.deb` de `LautaroPiugh/lolterm` por HTTPS y exige SHA256. Sin token en el paquete.
- Electron en Linux puede ir con `--no-sandbox` por el helper `chrome-sandbox`; es una limitación de empaquetado, no el modelo final.
- No guardes secretos en `config.toml` sincronizable. SSH usa el agent/claves del sistema.

## Licencia

MIT. Ver `LICENSE`.
