# LolTerm

Multiplexor gráfico de terminal: panes, tabs, PTY local o SSH remoto.
No reemplaza nvim, lazygit ni tmux: los abre en una ventana (Electron). No orquesta agentes.

```text
ventana Electron → lolterm-core → PTY → cualquier CLI
ventana Electron → lolterm-core → PTY → ssh → tmux
lolterm CLI      → el mismo crate (config / workspaces, sin PTY todavía)
```

## CLI

```bash
cargo run -p lolterm-core --bin lolterm -- status
cargo run -p lolterm-core --bin lolterm -- workspace list
```

Lee `~/.config/lolterm/` (no habla con Electron). `status` muestra workspace actual, branch, máquinas y la sesión tmux remota que usaría este workspace. No imprime env ni secretos.

## Correr (GUI)

```bash
cd apps/desktop
npm install
npm run dev
```

Eso compila `lolterm-core`, levanta Vite y abre Electron (tema Sage). Paleta: `Ctrl-b` / `Ctrl-p`.

Brief de diseño (Figma Make / designer): `apps/desktop/figma-prompt.txt`.

## Identidad

El chrome es mint (activity rail `⌂ F ± > ☁`, explorer, tabs, status). El trabajo vive en xterm.js conectado a PTYs reales.

- `+` nueva tab · `‖` split horizontal · `☰` split vertical
- `/run` (paleta) abre nvim, lazygit, btop, claude, …
- `/files` busca y abre en `$EDITOR`
- `/ssh` y `/ts-ssh` piden destino/usuario; la password la pide ssh en el pane

## Remoto

```toml
# ~/.config/lolterm/config.toml
[ui]
theme = "sage" # sage | dusk | mono

[remote]
user = "chae"
tmux = "lolterm" # prefijo; vacío = ssh sin tmux. sesión = lolterm-<workspace>

[[machines]]
name = "chae"
target = "chae.tailnet.ts.net"
user = "chae"
kind = "tailscale"
```

Conectar (sidebar Remoto, `/ssh`, `/ts-ssh`) abre:

```text
PTY → ssh -tt dest → tmux new-session -A -s lolterm-<workspace>
```

Si no hay tmux en el host, cae al shell de login. La máquina queda en el registro (máx. 12). Cada workspace usa su propia sesión remota para no pisarse.

## Config y sesión

- Config: `~/.config/lolterm/config.toml`
- Sesión: `~/.config/lolterm/session.toml`
