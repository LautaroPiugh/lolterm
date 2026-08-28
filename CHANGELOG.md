# Changelog

Los cambios versionados los escribe [Release Please](https://github.com/googleapis/release-please) a partir de Conventional Commits.

## [0.13.4](https://github.com/LautaroPiugh/lolterm/compare/v0.13.3...v0.13.4) (2026-08-28)

### Bug Fixes

* harden HTTP LAN config parsing, password handling, workspace path confinement, and live context refresh
* prevent implicit Electron Builder publishing during CI packaging
* block OSC 52 clipboard readback and tighten Desktop external URL handling

## [0.13.3](https://github.com/LautaroPiugh/lolterm/compare/v0.13.2...v0.13.3) (2026-08-24)

### Bug Fixes

* conservar el resultado de instalaciones de herramientas después de cerrar su PTY
* hacer visible el worktree de agentes e integrar su rama solo mediante fast-forward seguro

## [0.13.2](https://github.com/LautaroPiugh/lolterm/compare/v0.13.1...v0.13.2) (2026-08-23)

* agent catalog expansion and quota handling improvements

## [0.13.1](https://github.com/LautaroPiugh/lolterm/compare/v0.13.0...v0.13.1) (2026-08-22)


### Bug Fixes

* **desktop:** regenerate icon sizes with electron and ship all hicolor sizes in the deb ([2ad6f60](https://github.com/LautaroPiugh/lolterm/commit/2ad6f60406177a61e310ca5d118eda1debb7826f))
* **desktop:** ship suid chrome-sandbox, root-owned deb, and appstream metainfo ([fb5fc4b](https://github.com/LautaroPiugh/lolterm/commit/fb5fc4b49e602f9f2edc137699c75129a8b6c708))

* add Hermes, Goose, Aider, Crush, Qwen Code, and OpenHands agent launchers
* restore Copilot quota visibility and warm quota workers at sidecar startup
* make the quota menu scrollable and prioritize installed CLIs

## [0.13.0](https://github.com/LautaroPiugh/lolterm/compare/v0.12.0...v0.13.0) (2026-08-21)


### Features

* tighten quota, local diagnostics, and theme previews ([b105d28](https://github.com/LautaroPiugh/lolterm/commit/b105d285133b5470a0b40a473572ddc28d58842f))


### Bug Fixes

* show a themed splash and survive a missed core ready event ([1f9864e](https://github.com/LautaroPiugh/lolterm/commit/1f9864e450456706b7d5e0edd99e4cf1c7e01def))

## [0.12.0](https://github.com/LautaroPiugh/lolterm/compare/v0.11.0...v0.12.0) (2026-08-21)


### Features

* add a CodeMirror file overlay, workspace fs ops, and icon sync ([c960097](https://github.com/LautaroPiugh/lolterm/commit/c9600977cb1e8f30a6bde8cbb086893eb639ed74))

## [0.11.0](https://github.com/LautaroPiugh/lolterm/compare/v0.10.0...v0.11.0) (2026-08-21)

### Features

* barra de estado con puertos, procesos y atención de agentes; temas Claro, Oscuro, Contraste, Tide y Ember en todo el chrome
* overlay para leer/guardar un archivo del workspace; REST client para `.http`/`.rest` (secretos desde `.env` local)
* HTTP LAN opt-in (password en data_dir, sin TLS propio)
* vista Git tipo SCM: staged/changes, commit, fetch, pull `--ff-only` (sin merge ni force-push); lazygit sigue disponible
* panel de Ajustes (temas, layouts, entorno, HTTP) e instalación de CLIs conocidas en un PTY
* abrir nvim, lazygit, btop, yazi, agentes y el resto del catálogo desde paleta, Inicio o Ajustes → Abrir
* `gh` y `rg` se instalan desde Ajustes; no aparecen en el `+` (no son una tab vacía)
* cuota de GitHub Copilot y copiar al seleccionar en la terminal (`/copy-select`)
* barra táctil de teclas de terminal cuando el chrome es estrecho

## [Unreleased]

## [0.10.0](https://github.com/LautaroPiugh/lolterm/compare/v0.9.0...v0.10.0) (2026-08-20)


### Features

* add Orquester-style quota, media chip, and palettes ([66c661e](https://github.com/LautaroPiugh/lolterm/commit/66c661e9075499fe422c5b80809a595d3aaa4460))
* **desktop:** add Ubuntu .deb updates and prepare a public repo ([184c292](https://github.com/LautaroPiugh/lolterm/commit/184c292a8fa9b81a0749ded7b87020e6328a5fe2))
* **quota:** show OpenCode Go and ClinePass subscription windows ([11c3ac5](https://github.com/LautaroPiugh/lolterm/commit/11c3ac55f0c251677456a188aa63eacbe90e2eef))
* **ssh:** keep sessions alive and read Include from ssh config ([db8202f](https://github.com/LautaroPiugh/lolterm/commit/db8202f0dfc3c3aaa2829755ea9bc126d6dfe1b7))
* **ssh:** reconnect once and restore keepalives; remember window size ([cd76c16](https://github.com/LautaroPiugh/lolterm/commit/cd76c16975397aa1e54cd3c331c7c79af5f94e90))
* **ui:** replace paired palettes with four independent themes ([81f59e3](https://github.com/LautaroPiugh/lolterm/commit/81f59e3d8f867198c1f55aad766d4493e66fae28))

## [0.9.0](https://github.com/LautaroPiugh/lolterm/compare/v0.8.0...v0.9.0) (2026-08-17)

Era **Stabilization & Distribution**. Primera entrega: sidecar estable y artefactos Linux. Sin auto-update, firmas ni macOS/Windows.

### Features

* IPC: método desconocido y JSON inválido no cuelgan el `invoke` (error con `id` real, o evento `core-error`)
* si `lolterm-core` se cae, Electron reintenta (timeout 8s, flush de pending, aviso en la barra)
* al cerrar un pane en Linux se manda `SIGHUP` al process group del PTY
* tag `v*` empaqueta AppImage + `.deb` y los adjunta a la GitHub Release con `SHA256SUMS.txt`

## [0.8.0](https://github.com/LautaroPiugh/lolterm/compare/v0.7.0...v0.8.0) (2026-08-17)

Era **Extensibility**. Superficie estable en TOML local; no hay VM de plugins ni paneles React custom.

### Features

* `~/.config/lolterm/commands.toml` — comandos `ext.<slug>` que abren un binario (`program_ok`, args sin flags/paths)
* `hooks.toml` — solo `on = "workspace.open"`; no relanza si el programa ya está abierto
* `themes/*.toml` — temas `#RRGGBB` (no pisan sage/dusk/mono)
* `status.toml` — primera línea de un archivo o stdout de un programa (máx. 40 chars)
* `context.toml` — JSON string:string → `context.extra` (sin keys/valores que parezcan secretos)
* `extensions/<nombre>/extension.toml` — empaqueta lo anterior
* paleta, barra de estado, picker de temas e Inicio leen el bundle
* editor en Desktop (`/commands`, Ctrl-Alt-,) para `commands.toml` y atajos; doble clic graba la combinación y avisa si ya está en uso
* el explorer lista carpetas/archivos ocultos (sigue omitiendo `.git`, `node_modules`, `target`, …)

## [0.7.0](https://github.com/LautaroPiugh/lolterm/compare/v0.6.0...v0.7.0) (2026-08-17)

Era **AI Environment**. LoLTerm organiza agentes CLI; no implementa chat ni runtime.

### Features

* cada PTY recibe `LOLTERM_CONTEXT` / `LOLTERM_ROOT` / `LOLTERM_WORKSPACE`
* agentes (codex, claude, opencode, gemini, cline) abren en `git worktree` bajo `~/.local/share/lolterm/worktrees/` (`LOLTERM_WORKTREE`)
* status de agentes en la barra, historial en Inicio, aviso al cerrar
* paleta: `/codex`, `/claude`, `/opencode`, …
* `[ai] worktrees = false` en `config.toml` si se quiere el working tree principal

## [0.6.0](https://github.com/LautaroPiugh/lolterm/compare/v0.5.2...v0.6.0) (2026-08-17)

Era **Context Layer**. `lolterm context` / `panes` / `processes` leen el mux en vivo si el Desktop está abierto.

### Features

* Unix socket de solo lectura (`$XDG_RUNTIME_DIR/lolterm/mux.sock`) entre la CLI y `lolterm-core`
* JSON de contexto con `"live": true` (cwd del pane enfocado, procesos, panes) o fallback a `session.toml` con `"live": false`
* el JSON nunca incluye valores de env ni keys que parezcan secretos

## [0.5.2](https://github.com/LautaroPiugh/lolterm/compare/v0.5.1...v0.5.2) (2026-08-17)

Uso diario: `npm run dev`. El icono de escritorio queda en el `.deb`/AppImage; en dev GNOME/Wayland puede seguir mostrando el de Electron.

### Features

* Inicio al cerrar todas las pestañas (workspace, CLIs, máquinas, layouts)
* el `+` de tabs pregunta qué abrir (shell, SSH, agentes) y recuerda el default de `Ctrl-Alt-N`
* arrastrar una pestaña al borde parte el layout
* icono LoLTerm (panes + prompt mint) para ventana, `.desktop` y titlebar

### Bug Fixes

* el área de panes volvía a medir la mitad de la ventana después de Inicio
* el menú de `+` se recortaba al borde derecho

## [0.5.1](https://github.com/LautaroPiugh/lolterm/compare/v0.5.0...v0.5.1) (2026-08-17)

Uso diario: `npm run dev`. `v0.5.0` quedó en el árbol pero no se etiquetó en GitHub.

### Features

* `Ctrl-Tab` / `Ctrl-Shift-Tab` cicla tabs (Electron intercepta el atajo de Chromium)
* clic en el nombre del titlebar abre Inicio; de nuevo cicla workspaces (`Ctrl-Alt-[` `]`)
* Inicio muestra los comandos “al abrir”

### Bug Fixes

* restaurar un layout ya no degrada nvim/lazygit a shell y duplica el startup

## [0.5.0](https://github.com/LautaroPiugh/lolterm/compare/v0.4.1...v0.5.0) (2026-08-16)

Era **LoLTerm CLI**. El empaquetado Linux existe; el uso diario sigue siendo `npm run dev`.

### Features

* CLI `lolterm` como control del mismo core: `.`, `workspace list/open/forget`, `ssh`, `run`, `status`
* `lolterm context` (JSON) y tablas `workspace current`, `panes`, `processes`, `machines` (sesión guardada, sin secretos)

## [0.4.1](https://github.com/LautaroPiugh/lolterm/compare/v0.3.0...v0.4.1) (2026-08-16)

### Features

* empaquetado Linux: AppImage y `.deb` con sidecar `lolterm-core`

### Bug Fixes

* ventana en blanco al abrir el paquete (`file://` + `crossorigin` de Vite)
* el core ya no toma el path del ejecutable como workspace

## [0.4.0](https://github.com/LautaroPiugh/lolterm/compare/v0.3.0...v0.4.0) (2026-08-16)

### Features

* registro de máquinas, SSH/Tailscale y sesión tmux `lolterm-<workspace>`
* CLI `lolterm`: status, workspaces, `ssh`, `run` y abrir/enfocar el Desktop
* `lolterm` sin argumentos abre el workspace activo; `pending.toml` habla con la instancia ya abierta

## [0.3.0](https://github.com/LautaroPiugh/lolterm/compare/v0.2.0...v0.3.0) (2026-08-15)

### Features

* variables de entorno por workspace (Home → Inicio), aplicadas a PTYs nuevos
* `LOLTERM_ROOT` sigue siendo la raíz del workspace y no se deja sobrescribir

## [0.2.0](https://github.com/LautaroPiugh/lolterm/compare/v0.1.0...v0.2.0) (2026-08-15)

### Features

* clipboard, paste, OSC 52 y cleanup al salir de un pane
* registry de comandos, keybindings y zoom/nav/swap de panes
* duplicar tabs y presets de layout (nvim+shell, stack, ide)
* workspaces persistentes con comandos de arranque
* la versión se muestra en titlebar, status y Home (`v0.2.0`)

### Bug Fixes

* Ctrl-B y atajos ignorados por el textarea de xterm
* pestaña negra al volver: el emulador ya no se destruye al cambiar de tab
* ruido de Chromium (VSync / X11) filtrado en `npm run dev`

EOF
