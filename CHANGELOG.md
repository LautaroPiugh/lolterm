# Changelog

Los cambios versionados los escribe [Release Please](https://github.com/googleapis/release-please) a partir de Conventional Commits.

## [0.15.0](https://github.com/LautaroPiugh/lolterm/compare/v0.14.0...v0.15.0) (2026-08-31)


### Features

* add a CodeMirror file overlay, workspace fs ops, and icon sync ([c960097](https://github.com/LautaroPiugh/lolterm/commit/c9600977cb1e8f30a6bde8cbb086893eb639ed74))
* add clipboard, pane cleanup, and mux command registry ([cb6f557](https://github.com/LautaroPiugh/lolterm/commit/cb6f557e34a24aed94d2fd4ff3209de0db17a421))
* add Copilot quota and copy-on-select clipboard ([22a6c7a](https://github.com/LautaroPiugh/lolterm/commit/22a6c7a2714f0928132bfb4bd8b1bfa2a74727f8))
* add Fedora rpm packaging, pnpm, agent API keys and worktree toggle ([0bd3fd7](https://github.com/LautaroPiugh/lolterm/commit/0bd3fd78d2824cc4114a5e99efa252ff7ff942ae))
* add machine registry, gear settings, and a visible terminal scrollbar ([72843cd](https://github.com/LautaroPiugh/lolterm/commit/72843cdd8deb41402b3a79e091b747d58de774f7))
* add minimal tui with alternate screen ([981e309](https://github.com/LautaroPiugh/lolterm/commit/981e309bcaba6fe02df130eedf0b3fe2d418b875))
* add multiplexer with workspaces, splits, mouse, and scrollback ([96625e1](https://github.com/LautaroPiugh/lolterm/commit/96625e1cbb3cec7ed2343e053bfa4343a172f311))
* add Orquester-style quota, media chip, and palettes ([66c661e](https://github.com/LautaroPiugh/lolterm/commit/66c661e9075499fe422c5b80809a595d3aaa4460))
* add SCM-style git view and a settings panel for themes and CLIs ([70f6f2b](https://github.com/LautaroPiugh/lolterm/commit/70f6f2b0a3d4198a2ebc76a18e4d9febf77417d4))
* add ssh panes, git status, and configurable chrome ([8496c4a](https://github.com/LautaroPiugh/lolterm/commit/8496c4adb28b81127de83e52dcae073c3e3e1ef6))
* add workspace HUD chips, overlays, and daily themes ([06211dc](https://github.com/LautaroPiugh/lolterm/commit/06211dc696c08fcd199b1c9a362d3b80d9a35c49))
* **ai:** host agent CLIs in git worktrees with live context ([80093d6](https://github.com/LautaroPiugh/lolterm/commit/80093d6f988dbb32e9a5f8ce6ba28fe9e633b974))
* **cli:** add inspect commands for current workspace, panes, processes, and machines ([daa714b](https://github.com/LautaroPiugh/lolterm/commit/daa714b660fbb21e9de87f467f2d3442899304dc))
* **cli:** add lolterm context JSON for other tools ([bf83cf4](https://github.com/LautaroPiugh/lolterm/commit/bf83cf40e8b6e4f93955389d55d4f6f2cce27101))
* **cli:** add lolterm status and workspace list ([bd531f4](https://github.com/LautaroPiugh/lolterm/commit/bd531f47231d1e5cc096af2a4d557be627c0233f))
* **cli:** open LoLTerm from ssh, workspace, and run ([18d7e86](https://github.com/LautaroPiugh/lolterm/commit/18d7e860ab7102269e63fa76042dbdafd9e1beb3))
* **cli:** open LoLTerm when invoked with no arguments ([32cc646](https://github.com/LautaroPiugh/lolterm/commit/32cc6469cab921181aa204b7ed23d7c7027ce3dc))
* **cli:** register workspaces without nesting subfolders ([2a2033c](https://github.com/LautaroPiugh/lolterm/commit/2a2033cda29486cbe42177b4ef0ce94661493c12))
* **context:** query the live mux over a read-only unix socket ([880ab2b](https://github.com/LautaroPiugh/lolterm/commit/880ab2b978c2246d597aa3b4aa0df1a3e501bd76))
* **core:** stabilize sidecar IPC and pack Linux artifacts on tag ([5d92bc2](https://github.com/LautaroPiugh/lolterm/commit/5d92bc2b20a1c7c01e3d97d23776e01635c17061))
* **desktop:** add start page, new-tab picker, and app icon ([634404b](https://github.com/LautaroPiugh/lolterm/commit/634404bb032edcdb57d1ba2a8f6d326bdc5b33e9))
* **desktop:** add Ubuntu .deb updates and prepare a public repo ([184c292](https://github.com/LautaroPiugh/lolterm/commit/184c292a8fa9b81a0749ded7b87020e6328a5fe2))
* **ext:** add TOML extensions and an in-app commands editor ([88607f1](https://github.com/LautaroPiugh/lolterm/commit/88607f1568c918fa0f962d5de9668460bc936274))
* forward keyboard input to the pty ([0675a8d](https://github.com/LautaroPiugh/lolterm/commit/0675a8dadb8e063af71b011e9f120e2010957320))
* launch catalog CLIs from the UI and keep gh/rg out of + ([56df273](https://github.com/LautaroPiugh/lolterm/commit/56df27342f1145a116ee18c01c5fc07f9da65ae8))
* **mux:** add builtin layout presets ([45f9414](https://github.com/LautaroPiugh/lolterm/commit/45f941476cf55c4483ca02474b43c0fb6c130696))
* **mux:** cycle tabs with Ctrl-Tab ([d89f987](https://github.com/LautaroPiugh/lolterm/commit/d89f9876ef5e6709695116a74b7553528943b9a0))
* **mux:** duplicate tabs with a fresh set of PTYs ([42cad27](https://github.com/LautaroPiugh/lolterm/commit/42cad272e89b5f189fa1076433989d6d8b6daf9e))
* persist shell cwd and close panes on exit ([bf9ee2a](https://github.com/LautaroPiugh/lolterm/commit/bf9ee2ab3210906523ca968ce73e33aaa903d0df))
* persist workspaces with startup commands and keep tab buffers ([b213b0e](https://github.com/LautaroPiugh/lolterm/commit/b213b0e40b020533b96c53b23399b82a34f75bad))
* **quota:** show OpenCode Go and ClinePass subscription windows ([11c3ac5](https://github.com/LautaroPiugh/lolterm/commit/11c3ac55f0c251677456a188aa63eacbe90e2eef))
* **remote:** attach SSH to tmux and remember machines on connect ([abed90d](https://github.com/LautaroPiugh/lolterm/commit/abed90db2ba9595fdbf1e047df2e29b744d90bd0))
* **remote:** mark SSH tabs and use a tmux session per workspace ([25a4c3a](https://github.com/LautaroPiugh/lolterm/commit/25a4c3a899120f36b64e25c3c0d1d90d3c3c3065))
* render pty output with tui-term ([8cdf7d7](https://github.com/LautaroPiugh/lolterm/commit/8cdf7d717329242f09163df6903d0e7b2d86cbe1))
* resize pty and vt100 screen with the window ([d56eb7b](https://github.com/LautaroPiugh/lolterm/commit/d56eb7b079b13ef0c1f4e74fd38393e721989124))
* show version, restore pane programs, and fix Ctrl-B ([5baf630](https://github.com/LautaroPiugh/lolterm/commit/5baf630047b260f1a56d597c759e741a4f68f1b5))
* spawn user shell inside a pty ([20b4130](https://github.com/LautaroPiugh/lolterm/commit/20b413007481e93c24d2f323e10f662c0ba6dd8d))
* **ssh:** keep sessions alive and read Include from ssh config ([db8202f](https://github.com/LautaroPiugh/lolterm/commit/db8202f0dfc3c3aaa2829755ea9bc126d6dfe1b7))
* **ssh:** reconnect once and restore keepalives; remember window size ([cd76c16](https://github.com/LautaroPiugh/lolterm/commit/cd76c16975397aa1e54cd3c331c7c79af5f94e90))
* tighten quota, local diagnostics, and theme previews ([b105d28](https://github.com/LautaroPiugh/lolterm/commit/b105d285133b5470a0b40a473572ddc28d58842f))
* **ui:** replace paired palettes with four independent themes ([81f59e3](https://github.com/LautaroPiugh/lolterm/commit/81f59e3d8f867198c1f55aad766d4493e66fae28))
* **workspace:** add portable catalog, navigation, and project metadata ([ecdc94b](https://github.com/LautaroPiugh/lolterm/commit/ecdc94b3b5437a6f27f73cc5395c338d154ae289))
* **workspace:** open catalog from the titlebar and skip duplicate startup ([a5b027a](https://github.com/LautaroPiugh/lolterm/commit/a5b027a04f32bcf39f39cf31aa9a726968a59a3b))
* **workspace:** persist per-workspace environment variables ([b435458](https://github.com/LautaroPiugh/lolterm/commit/b435458f0b4e21601aaceb834559b2234c638dfe))


### Bug Fixes

* **desktop:** regenerate icon sizes with electron and ship all hicolor sizes in the deb ([2ad6f60](https://github.com/LautaroPiugh/lolterm/commit/2ad6f60406177a61e310ca5d118eda1debb7826f))
* **desktop:** ship suid chrome-sandbox, root-owned deb, and appstream metainfo ([fb5fc4b](https://github.com/LautaroPiugh/lolterm/commit/fb5fc4b49e602f9f2edc137699c75129a8b6c708))
* **desktop:** use electron-builder 26 linux.desktop.entry schema ([1b75c1f](https://github.com/LautaroPiugh/lolterm/commit/1b75c1fef8fd35cbf073a4db5ba3ee5134d7d579))
* harden critical release and runtime paths ([276bed5](https://github.com/LautaroPiugh/lolterm/commit/276bed5adc941cfa6ad0957256ac8f51191bbff5))
* harden desktop release surface ([4937b8a](https://github.com/LautaroPiugh/lolterm/commit/4937b8a5ed5b63a14e284aa32023dbc4a41684db))
* improve agent catalog and quota handling ([e72e5cb](https://github.com/LautaroPiugh/lolterm/commit/e72e5cb4323e088f6da970ec806df2e25eff8b86))
* retain installation results and integrate agent worktrees ([1170b48](https://github.com/LautaroPiugh/lolterm/commit/1170b48a033fc09696cef6edb315722dbe857b9f))
* show a themed splash and survive a missed core ready event ([1f9864e](https://github.com/LautaroPiugh/lolterm/commit/1f9864e450456706b7d5e0edd99e4cf1c7e01def))

## [0.14.0](https://github.com/LautaroPiugh/lolterm/compare/v0.13.4...v0.14.0) (2026-08-31)

### Features

* empaquetar `.rpm` para Fedora junto al `.deb` y subir `SHA256SUMS.txt` en cada release
* auto-update detecta la distro y actualiza `.deb` o `.rpm` (apt/dnf) verificando SHA256
* guardar API keys de agentes machine-local (`secrets.json`, solo se inyectan en panes de agente)
* agregar agentes pi, omp y omh al catálogo
* preferencia de abrir agentes en worktree o directorio real, editable desde Ajustes
* botón para cerrar un pane desde el chrome de la terminal

### Build System

* migrar npm a pnpm (`pnpm-lock.yaml` y `pnpm-workspace.yaml`)

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
