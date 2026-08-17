# LolTerm

Multiplexor gráfico de terminal: panes, tabs, PTY local o SSH remoto.
No reemplaza nvim, lazygit ni tmux: los abre en una ventana (Electron). No orquesta agentes.

```text
ventana Electron → lolterm-core → PTY → cualquier CLI
ventana Electron → lolterm-core → PTY → ssh → tmux
lolterm CLI      → el mismo crate (config / workspaces) y abre o enfoca el Desktop
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

Lee y escribe `~/.config/lolterm/` (`pending.toml` para hablar con una instancia ya abierta). Sin argumentos, `.`, `workspace open`, `ssh` y `run` abren o enfocan el Desktop. `context` es JSON para otras herramientas; `workspace current`, `panes`, `processes` y `machines` son tablas del último layout guardado (no el mux en vivo). No imprime env, secretos ni args de panes.

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

Eso compila `lolterm-core`, levanta Vite y abre Electron (tema Sage). Paleta: `Ctrl-b` / `Ctrl-p`. Tabs: `Ctrl-Tab` / `Ctrl-Shift-Tab`. Splits: `Ctrl-Alt-v` (derecha) y `Ctrl-Alt-s` (abajo). Restart del pane: `Ctrl-Alt-r`. Renombrar tab: `Ctrl-Alt-e`.

## Empaquetar (Linux)

```bash
cd apps/desktop
npm run pack
```

Quedan archivos en `apps/desktop/release/`. El sidecar Rust va en `resources/`, no dentro del asar.

**AppImage** — no se instala. Es un archivo ejecutable (útil para probar o copiar a otra máquina):

```bash
chmod +x apps/desktop/release/LoLTerm-0.5.0-linux-x86_64.AppImage
./apps/desktop/release/LoLTerm-0.5.0-linux-x86_64.AppImage
```

**.deb** — es el formato de Ubuntu/Debian. Instala LoLTerm en el menú de aplicaciones y un `lolterm` en `/usr/bin`:

```bash
sudo apt install ./apps/desktop/release/LoLTerm-0.5.0-linux-amd64.deb
```

Si `apt` se queja de dependencias:

```bash
sudo dpkg -i apps/desktop/release/LoLTerm-0.5.0-linux-amd64.deb
sudo apt-get install -f
```

En desarrollo, Electron sigue usando `target/debug/lolterm-core`. El `lolterm` de `cargo install` en `~/.local/bin` gana a `/usr/bin` si `~/.local/bin` está primero en PATH.

El `.deb` es una foto fija: no se actualiza solo cuando cambiás el código. Para el día a día usá `npm run dev`. Para refrescar la app instalada, volvé a empaquetar e instalá el `.deb` nuevo. El auto-update (descargar la última release de GitHub) viene después, cuando haya tags/releases publicados.

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
