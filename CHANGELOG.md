# Changelog

Los cambios versionados los escribe [Release Please](https://github.com/googleapis/release-please) a partir de Conventional Commits. Las secciones 0.2.0–0.4.0 se publicaron a mano para alinear la versión global.

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
