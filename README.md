# LoLTerm

LoLTerm es un **workspace local-first de terminales**: una aplicación desktop para organizar proyectos, panes, pestañas, shells, editores, Git, SSH/tmux y agentes CLI sin reemplazarlos.

> LoLTerm es el entorno desde el que trabajás; las herramientas siguen siendo las herramientas.

Hoy el paquete oficial es un **`.deb` para Ubuntu/Debian**, un **`.rpm` para Fedora** y un **`.AppImage` portable**. Linux es la plataforma principal. macOS, Windows, apt repo y firmas GPG quedan para una etapa posterior.

## Qué hace

- Abre procesos reales dentro de **PTYs**: `bash`, `nvim`, `lazygit`, `btop`, `fzf`, `yazi`, `ssh`, `tmux`, Codex, Claude Code, OpenCode y otras CLIs.
- Organiza esos procesos en pestañas, panes y workspaces.
- Expone contexto local con `lolterm context` y `LOLTERM_CONTEXT`.
- Usa el `ssh` del sistema y puede recuperar sesiones remotas con `tmux`.
- Incluye overlays de conveniencia: explorer/editor simple, Git básico, REST client, Ajustes, temas, comandos y atajos.
- Mantiene la configuración en archivos locales TOML.

## Qué no es

LoLTerm **organiza** herramientas existentes. No las reemplaza.

| LoLTerm no es… | Lo que hace en su lugar |
| --- | --- |
| un IDE tradicional | overlay para leer/guardar archivos; el editor serio sigue siendo `nvim` / `$EDITOR` en un PTY |
| un cliente Git completo | vista tipo SCM para stage/commit/fetch/pull `--ff-only`; `lazygit` sigue disponible |
| un chat o runtime de IA | abre agentes CLI en terminales, con contexto y worktrees cuando aplica |
| un fork de tmux | puede usar tmux en remoto para no perder sesiones |
| un cliente SSH propio | usa `ssh`, claves y agent del sistema |
| un marketplace | Ajustes ejecuta comandos oficiales de instalación en un PTY |

## Estado

| Área | Estado actual |
| --- | --- |
| Terminal/PTY | usable con procesos interactivos reales; sigue siendo el centro del producto |
| Multiplexer | pestañas, splits, resize, foco y comandos básicos |
| Workspaces | raíz, layouts, startup commands y contexto local |
| Remote | SSH/Tailscale/tmux en desarrollo activo |
| CLI | `lolterm`, `context`, `panes`, `processes`, `workspace`, `ssh`, `run` |
| Distribución | `.deb` (Ubuntu/Debian), `.rpm` (Fedora) y `.AppImage` (portable) en GitHub Releases |
| Updates | `/update` detecta distro (deb/rpm) o AppImage y verifica SHA256 |
| Seguridad | local-first; HTTP LAN es opt-in y sin TLS propio |

## Arquitectura corta

```text
teclado/mouse
    ↓
React + xterm.js
    ↓
preload IPC
    ↓
Electron main
    ↓
JSON line protocol
    ↓
lolterm-core (Rust)
    ↓
PTY real
    ↓
proceso CLI
```

Tres piezas se ven como “LoLTerm”:

1. **Desktop** — ventana, tabs, splits, explorer, Git, REST, Ajustes, temas.
2. **Core Rust** — crea PTYs, controla procesos, mantiene mux/sesión/workspaces, SSH y contexto.
3. **CLI `lolterm`** — abre workspaces/comandos y consulta contexto desde otra terminal.

Un **PTY** (*pseudo terminal*) hace que un proceso interactivo crea que está conectado a una terminal real. Por eso importan resize, raw mode, alternate screen, mouse events, Unicode y secuencias ANSI/VT.

Más detalle: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Instalar en Ubuntu/Debian

1. Bajá el `.deb` de la [última release](https://github.com/LautaroPiugh/lolterm/releases/latest).
2. Instalalo:

```bash
sudo apt install ./LoLTerm-*-linux-amd64.deb
```

Queda en el menú de aplicaciones y deja `lolterm` en el PATH.

Actualizar:

- desde la paleta: `/update`;
- o desde el aviso cuando hay una versión nueva.

El updater baja el `.deb` de la release latest, verifica `SHA256SUMS.txt` y recién después instala. No hay apt repo ni firma GPG todavía. Detalles: [`docs/RELEASE.md`](docs/RELEASE.md) y [`SECURITY.md`](SECURITY.md).

## Instalar en Fedora

1. Bajá el `.rpm` de la [última release](https://github.com/LautaroPiugh/lolterm/releases/latest).
2. Instalalo:

```bash
sudo dnf install ./LoLTerm-*-linux-x86_64.rpm
```

Queda en el menú de aplicaciones y deja `lolterm` en el PATH. Después podés actualizar desde la paleta (`/update`) o desde el aviso cuando hay versión nueva.

## Instalar como AppImage (portable)

1. Bajá el `.AppImage` de la [última release](https://github.com/LautaroPiugh/lolterm/releases/latest).
2. Hacelo ejecutable y corrélo:

```bash
chmod +x LoLTerm-*-linux-x86_64.AppImage
./LoLTerm-*-linux-x86_64.AppImage
```

No se instala; es un archivo auto-contenido. Para integrarlo al menú podés usar [AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher) o `appimaged`. El auto-update reemplaza el archivo y reinicia la app.

## Uso básico

| Acción | Cómo |
| --- | --- |
| Paleta | `Ctrl-b` o `Ctrl-p` |
| Cambiar de tab | `Ctrl-Tab` / `Ctrl-Shift-Tab` |
| Split derecha | `Ctrl-Alt-v` |
| Split abajo | `Ctrl-Alt-s` |
| Workspaces | clic en el nombre, o `Ctrl-Alt-[` / `Ctrl-Alt-]` |
| Reiniciar pane | `Ctrl-Alt-r` |
| Ajustes | engranaje, `/settings` o `/theme` |
| Comandos y atajos | `Ctrl-Alt-,` o `/commands` |
| Git | rail Git: staged/changes, commit con `Ctrl+Enter`, fetch, pull `--ff-only` |
| Archivo | explorer + overlay, o `nvim` / `$EDITOR` |
| REST | `+` → REST, o `/rest`; secretos desde `.env` local |
| Copiar al seleccionar | `/copy-select` o `Ctrl-Shift-C` |

El botón `+` permite abrir shells, SSH, nvim, lazygit, btop, yazi, agentes y otras CLIs conocidas.

## CLI

```bash
lolterm .
lolterm workspace list
lolterm workspace open <nombre>
lolterm ssh home
lolterm run nvim
lolterm context
lolterm panes
lolterm processes
```

`lolterm context` expone una foto del workspace: carpeta, rama Git, procesos, panes y archivo enfocado cuando se puede detectar. No debe incluir valores completos de variables de entorno ni claves que parezcan secretos.

## Máquinas remotas

LoLTerm usa el `ssh` del sistema. Las contraseñas/passphrases las pide `ssh` dentro del PTY.

Ejemplo de `~/.config/lolterm/config.toml`:

```toml
[remote]
user = "dev"
tmux = "lolterm"

[[machines]]
name = "home"
target = "home.example.ts.net"
user = "dev"
kind = "tailscale"
```

Si el enlace se corta, LoLTerm intenta reconectar una vez. Con `tmux -A`, la sesión remota puede sobrevivir al corte.

## Configuración

| Ruta | Qué contiene |
| --- | --- |
| `~/.config/lolterm/` | config portable: temas, atajos, comandos, workspaces, extensiones TOML |
| `$XDG_RUNTIME_DIR/lolterm/` | estado vivo: socket del mux y contexto temporal |
| `$XDG_DATA_HOME/lolterm/` | estado local: worktrees, historial local, password HTTP |

No guardes secretos en archivos sincronizables. Para REST, publicá sólo `.env.example`; tu `.env` real queda ignorado por Git.

## Seguridad y privacidad

- No hay cuenta ni backend cloud de LoLTerm.
- No hay telemetría automática.
- HTTP LAN es opt-in, usa password local y no trae TLS propio.
- Los diagnósticos son locales; revisalos antes de pegarlos en un issue.
- Las CLIs externas que abras pueden usar su propia red/autenticación.

Leer:

- [`SECURITY.md`](SECURITY.md)
- [`PRIVACY.md`](PRIVACY.md)
- [`docs/PUBLICATION_CHECKLIST.md`](docs/PUBLICATION_CHECKLIST.md)

## Desarrollo

```bash
# Rust, desde la raíz
cargo fmt --all -- --check
cargo clippy -p lolterm-core --all-targets -- -D warnings
cargo test --workspace

# Desktop
cd apps/desktop
pnpm install --frozen-lockfile
pnpm run build
pnpm run dev
```

Guía completa: [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).
Contribuciones: [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Release

Release Please mantiene versiones y changelog. Los tags `v*` empaquetan Linux `.deb`, `.rpm` y `.AppImage`, y adjuntan `SHA256SUMS.txt` a GitHub Releases.

Ver [`docs/RELEASE.md`](docs/RELEASE.md).

## Licencia

[MIT](LICENSE).
