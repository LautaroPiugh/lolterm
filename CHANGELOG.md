# Changelog

Los cambios versionados los escribe [Release Please](https://github.com/googleapis/release-please) a partir de Conventional Commits. Las secciones 0.2.0–0.8.0 se publicaron a mano para alinear la versión global con las eras de `AGENTS.md`.

## [0.8.0](https://github.com/LautaroPiugh/lolterm/compare/v0.7.0...v0.8.0) (2026-08-17)

Era **Extensibility** (`AGENTS.md` v0.8.x). Superficie estable en TOML local; no hay VM de plugins ni paneles React custom.

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

Era **AI Environment** (`AGENTS.md` v0.7.x). LoLTerm organiza agentes CLI; no implementa chat ni runtime.

### Features

* cada PTY recibe `LOLTERM_CONTEXT` / `LOLTERM_ROOT` / `LOLTERM_WORKSPACE`
* agentes (codex, claude, opencode, gemini, cline) abren en `git worktree` bajo `~/.local/share/lolterm/worktrees/` (`LOLTERM_WORKTREE`)
* status de agentes en la barra, historial en Inicio, aviso al cerrar
* paleta: `/codex`, `/claude`, `/opencode`, …
* `[ai] worktrees = false` en `config.toml` si se quiere el working tree principal

## [0.6.0](https://github.com/LautaroPiugh/lolterm/compare/v0.5.2...v0.6.0) (2026-08-17)

Era **Context Layer** (`AGENTS.md` v0.6.x). `lolterm context` / `panes` / `processes` leen el mux en vivo si el Desktop está abierto.

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

Era **LoLTerm CLI** (`AGENTS.md` v0.5.x). El empaquetado Linux existe; el uso diario sigue siendo `npm run dev`.

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
