# Changelog

Los cambios versionados los escribe [Release Please](https://github.com/googleapis/release-please) a partir de Conventional Commits. Las secciones 0.2.0–0.5.1 se publicaron a mano para alinear la versión global con las eras de `AGENTS.md`.

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
