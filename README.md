# LolTerm

Multiplexor gráfico de terminal: panes, tabs, PTY local o SSH remoto.
No reemplaza nvim, lazygit ni tmux: los abre en una ventana (Electron). No orquesta agentes.

```text
ventana Electron → lolterm-core → PTY → cualquier CLI
ventana Electron → lolterm-core → PTY → ssh → tmux
lolterm CLI      → socket local al mux (context/panes/processes) o config / session.toml
```

## CLI

```bash
cargo run -p lolterm-core --bin lolterm
cargo run -p lolterm-core --bin lolterm -- status
cargo run -p lolterm-core --bin lolterm -- context
cargo run -p lolterm-core --bin lolterm -- workspace current
cargo run -p lolterm-core --bin lolterm -- panes
cargo run -p lolterm-core --bin lolterm -- processes
cargo run -p lolterm-core --bin lolterm -- machines
cargo run -p lolterm-core --bin lolterm -- workspace list
cargo run -p lolterm-core --bin lolterm -- .
cargo run -p lolterm-core --bin lolterm -- workspace open lolterm
cargo run -p lolterm-core --bin lolterm -- workspace forget desktop
cargo run -p lolterm-core --bin lolterm -- ssh
cargo run -p lolterm-core --bin lolterm -- ssh chae
cargo run -p lolterm-core --bin lolterm -- run nvim
```

Lee y escribe `~/.config/lolterm/` (`pending.toml` para hablar con una instancia ya abierta). Sin argumentos, `.`, `workspace open`, `ssh` y `run` abren o enfocan el Desktop. Con el Desktop abierto, `context`, `panes` y `processes` preguntan al mux por un Unix socket en el runtime dir (no en config sincronizable). Si no hay instancia, caen a `session.toml` y `context` lleva `"live": false`. Cada PTY recibe `LOLTERM_ROOT` y `LOLTERM_CONTEXT`. Un agente (codex/claude/opencode/…) abre en un `git worktree` bajo `~/.local/share/lolterm/worktrees/` y ve `LOLTERM_WORKTREE`. Desactivar: `[ai] worktrees = false` en `config.toml`.

Para tener `lolterm` en PATH (este repo, `~/.local/bin`):

```bash
cargo install --path crates/core --bin lolterm --force --root "$HOME/.local"
```

## Correr (GUI)

```bash
cd apps/desktop
npm install
npm run dev
```

Eso compila `lolterm-core`, levanta Vite y abre Electron (tema Sage). Paleta: `Ctrl-b` / `Ctrl-p`. Tabs: `Ctrl-Tab` / `Ctrl-Shift-Tab`. Workspaces: clic en el nombre del titlebar (Inicio) o `Ctrl-Alt-[` `]`. Splits: `Ctrl-Alt-v` (derecha) y `Ctrl-Alt-s` (abajo). Restart del pane: `Ctrl-Alt-r`. Renombrar tab: `Ctrl-Alt-e`.

## Empaquetar (Linux)

Los tags `v*` disparan CI (`.github/workflows/pack.yml`): AppImage, `.deb` y `SHA256SUMS.txt` van a la [GitHub Release](https://github.com/LautaroPiugh/lolterm/releases).

```bash
# local
cd apps/desktop
npm run pack
sha256sum release/LoLTerm-*-linux-*.AppImage release/LoLTerm-*-linux-*.deb
```

El sidecar Rust va en `resources/`, no dentro del asar.

**AppImage** — no se instala. Es un archivo ejecutable:

```bash
chmod +x apps/desktop/release/LoLTerm-0.9.0-linux-x86_64.AppImage
./apps/desktop/release/LoLTerm-0.9.0-linux-x86_64.AppImage
```

**.deb** — Ubuntu/Debian. Instala LoLTerm en el menú y un `lolterm` en `/usr/bin`:

```bash
sudo apt install ./apps/desktop/release/LoLTerm-0.9.0-linux-amd64.deb
```

Si `apt` se queja de dependencias:

```bash
sudo dpkg -i apps/desktop/release/LoLTerm-0.9.0-linux-amd64.deb
sudo apt-get install -f
```

En desarrollo, Electron usa `target/debug/lolterm-core`. El `lolterm` de `cargo install` en `~/.local/bin` gana a `/usr/bin` si `~/.local/bin` está primero en PATH.

El `.deb`/AppImage es una foto fija: no hay auto-update. Si `lolterm-core` se cae, Desktop reintenta un par de veces y avisa en la barra. La sesión restaurada **respawnea** procesos (no revive nvim/TUI con el buffer anterior).

Electron en Linux puede ir con `--no-sandbox` porque el helper SUID `chrome-sandbox` a menudo no conserva 4755 en una instalación de usuario. Es una particularidad de empaquetado, no el modelo de seguridad final.

Brief de diseño (Figma Make / designer): `apps/desktop/figma-prompt.txt`.

## Identidad

El chrome es mint (activity rail `⌂ F ± > ☁`, explorer, tabs, status). El trabajo vive en xterm.js conectado a PTYs reales.

- `+` nueva tab · cerrar todas las pestañas muestra Inicio (como VS Code)
- arrastrá una pestaña al borde del terminal para partir el layout
- clic en el nombre del titlebar abre Inicio; de nuevo cicla; `Ctrl-Alt-[` `]` también
- `/run` (paleta) abre nvim, lazygit, btop, claude, …
- `/files` busca y abre en `$EDITOR`
- `/ssh` y `/ts-ssh` piden destino/usuario; la password la pide ssh en el pane

## Remoto

```toml
# ~/.config/lolterm/config.toml
[ui]
theme = "sage" # sage | dusk | mono | id de themes/*.toml

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

## Extensiones (TOML)

No hay plugins JS. LoLTerm lee archivos locales en `~/.config/lolterm/`.

En Desktop, **Comandos y atajos** (engranaje, `/commands` o Ctrl-Alt-,) edita `commands.toml` y `keybindings.toml`. Es el equivalente al `keybindings.json` de VS Code: la UI escribe el archivo; también se puede abrir en `$EDITOR`.

```toml
# keybindings.toml — solo overrides; "" desactiva un default
[keys]
"ctrl+alt+b" = "ui.palette"
"ctrl+alt+z" = ""
```

```toml
# commands.toml
[[command]]
id = "ext.htop"
slash = "htop"
hint = "monitor"
run = "htop"

# hooks.toml — solo workspace.open; no relanza si ya está abierto
[[hook]]
on = "workspace.open"
run = "lazygit"

# status.toml
[[status]]
id = "branch-file"
file = ".git/HEAD"

# context.toml — JSON en el repo → context.extra
files = ["lolterm-context.json"]
```

```toml
# themes/nord.toml
id = "nord"
label = "Nord"
hint = "oscuro frío"
bg = "#2e3440"
fg = "#eceff4"
accent = "#88c0d0"
bar = "#3b4252"
pane = "#2e3440"
```

Un pack: `extensions/<nombre>/extension.toml` puede mezclar `name`, `[[command]]`, `[[hook]]`, `[[status]]`, `files` y `[theme]`. `run` tiene que pasar `program_ok` (sin `/` ni flags). Los args no pueden empezar con `-` ni contener `..` o `/`.

## Config y sesión

- Config: `~/.config/lolterm/config.toml`
- Sesión: `~/.config/lolterm/session.toml`
