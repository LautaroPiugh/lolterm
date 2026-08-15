# LolTerm

Multiplexor gráfico de terminal, local-first. Inspirado en tmux/Zellij y en el
producto de [@sammwy](https://x.com/sammwy) / [@vql3n](https://x.com/vql3n):
parece un IDE (chrome tipo VS Code/Cursor, mint Sage) pero **no es un IDE ni un
orquestador de agentes**. Abre CLIs en PTYs (local o SSH) y multiplexea el trabajo.

Ella lo dijo así: es más como tmux. No maneja agentes. Abrís las CLIs y, como
con ssh, las usás en esta PC o en un remoto. Agnóstico a qué corra adentro.

```text
ventana Electron (chrome Sage)
    → lolterm-core (Rust)
        → PTY → bash | nvim | lazygit | claude | ssh | lo que sea
```

Referencia visual: TUI mint
([bancan](https://x.com/sammwy/status/2087658292088545473)) → ventana de OS
([antes/después](https://x.com/vql3n/status/2087720358283514149),
[gatekeep](https://x.com/vql3n/status/2088333982316306920)).

## Regla fundamental de trabajo

No escribir código sin explicar. El autor aprende Rust **y** el stack del GUI
(Electron, React, TypeScript, xterm.js, IPC) mientras construye.

### Antes de modificar código, explicar

1. Qué problema se resuelve.
2. Qué concepto técnico interviene.
3. Qué significa cada sigla/término técnico.
4. Por qué elegimos una solución.
5. Qué alternativas razonables existen.
6. Qué archivos se van a modificar.
7. Cómo encaja en la arquitectura general.

### Después de implementar, explicar

1. Qué cambió.
2. Cómo funciona.
3. Partes importantes del código.
4. Sintaxis nueva (Rust o TS) que apareció.
5. Cómo probarlo (comandos concretos).
6. Qué errores podrían aparecer.
7. Qué aprendimos.

No dar por sentada la terminología. Explicar PTY, GUI, Electron (main vs
renderer), preload, IPC, sidecar, xterm.js, ANSI, JSON-RPC, ownership, borrowing,
channel, thread, event loop. Comparar con PHP/JS cuando ayude. No repetir
explicaciones ya dadas salvo un uso nuevo.

## Filosofía

1. **CLI agnostic.** Un pane es un proceso. No hay runtime de agentes, ni ACP,
   ni chat propio. Codex/Claude/nvim son el mismo objeto: `spawn` en un PTY.
2. **Multiplexor, no editor.** El chrome (rail, explorer, tabs, status) es de
   LolTerm. El contenido se edita en nvim/`$EDITOR` dentro del PTY. Monaco/LSP
   es fase posterior, no el producto ahora.
3. **Local-first.** Sin cuenta, nube, ni backend HTTP. Config y sesión en
   `~/.config/lolterm/`.
4. **Composable.** No reimplementar Git pesado, file manager ni tmux remoto.
   Overlay de `git log` ≠ cliente Git. `/ts-ssh` = PTY → ssh → tmux.
5. **Portable.** Linux primero (dev actual); macOS después; Windows más tarde.
6. **Solo GUI.** Ratatui/`crates/tui` se eliminó. El trabajo va a `apps/desktop` + `crates/core`. Prompt visual: `apps/desktop/figma-prompt.txt`.

## Stack

- **Rust** (edition 2024): `crates/core` — PTY bytes, mux, git, files, sesión,
  SSH/Tailscale. Binario sidecar `lolterm-core`.
- **Electron + React + TypeScript + Vite:** `apps/desktop` — chrome Sage.
- **xterm.js:** emulador VT en el DOM. Recibe bytes ANSI del core.
- **portable-pty:** el proceso cree que tiene una terminal de verdad.
- **Serde / TOML / JSON lines:** config, sesión, IPC.

No agregar dependencias sin explicar por qué. El core no usa Tokio; hilos +
canales. El GUI es el event loop de Chromium/React.

En Linux, `npm run dev` lanza Electron con `--no-sandbox` porque el helper SUID
`chrome-sandbox` no queda en 4755/root tras un `npm install` de usuario.

## Arquitectura

```text
lolterm/
├── crates/core/          # lib + bin lolterm-core (dueño de PTYs)
│   └── src/bin/lolterm-core.rs   # JSON-RPC por stdin/stdout
└── apps/desktop/         # Electron main/preload + React
    ├── electron/main.mjs
    ├── electron/preload.cjs
    ├── figma-prompt.txt  # brief para Figma Make
    └── src/              # App, xterm panes, tema Sage
```

```text
teclado/mouse (React)
    → IPC (preload)
    → Electron main
    → JSON line → lolterm-core
    → PTY write
PTY read → evento data (base64) → xterm.write
```

Remoto:

```text
LolTerm → PTY → ssh → MagicDNS → tmux new-session -A -s lolterm
```

**Main** = proceso Node con APIs de OS (spawnea el sidecar, diálogos).
**Renderer** = página web (React). **Preload** = puente seguro (`window.lolterm`).
**Sidecar** = proceso Rust; un crash del UI no debería ser el único dueño del PTY.

## Estado actual (2026-08-15)

El GUI **arranca**: ventana Sage, un PTY bash real, rail, inicio/abrir carpeta,
tabs `+`/`×`, splits, paleta Ctrl-b, explorer, overlay `git log`, status.

Huecos vs la referencia vql3n (próximos pasos, no un volcado):

- Titlebar nativa de GTK (File/Edit/View) en vez de chrome propio `— □ ×`.
- Rail/glifos y tabs tipo card todavía crudos (no el mint “IDE” del screenshot).
- Overlay git tapando el prompt; notices pegajosos (`solo hay una tab`).
- Splits sin arrastrar el divisor; un workspace a la vez.
- xterm: resize/focus/truecolor/nvim hay que endurecerlos (el test es nvim, no `ls`).
- Sesión: se persiste al salir; rehidratar layout+PTYs con más fidelidad.
- Electron en Linux: sandbox SUID; en dev usamos `--no-sandbox`.

Arranque:

```bash
cd apps/desktop && npm install && npm run dev   # GUI
cargo test -p lolterm-core
```

Paleta: `Ctrl-b` / `Ctrl-p`. Config: `~/.config/lolterm/config.toml`.
Sesión: `~/.config/lolterm/session.toml`.

## Próximos pasos (orden, un corte por vez)

No implementar esta lista de golpe. Cada ítem es una unidad con explicación +
`cargo test` / probar el GUI.

1. **Chrome de ventana.** `frame: false` o ocultar menú nativo; titlebar Sage
   (`lolterm`, workspaces, `— □ ×`). Que deje de parecer “Chromium genérico”.
2. **Rail y tabs.** Iconos `⌂ F ± > ☁` legibles; pills de tab como el screenshot
   (nombre + ×; `+` nueva). Default al explorer, no a Inicio vacío.
3. **Panes honestos.** Quitar o hacer dismissable el float de git; notices con
   TTL. Drag del split. Foco/resize de xterm estable (nvim, lazygit, `btop`).
4. **Explorer útil.** Files / Search como ahora; abrir archivo = `$EDITOR` en
   tab PTY; marcas git en el árbol; `/reveal`.
5. **Mux completo.** Varios workspaces; `/run` pulido; CLIs a pantalla completa
   en tab propia (nvim, lazygit, claude) como `tmux new-window`.
6. **Remoto.** `/ssh` y `/ts-ssh` con usuario; password en el PTY; recientes.
7. **Sesión.** Restaurar tabs, cwd de shells, proyecto al reabrir la app.
8. **Pulido.** Clipboard, copy-mode básico, tema dusk/mono además de Sage.

**Después (no ahora):** Monaco/LSP, música, yazi, daemon propio, Windows,
empaquetado `.deb`, Tauri si Electron pesa demasiado (el core Rust se reusa).

**Nunca (producto):** runtime de agentes, ACP, reescribir nvim o lazygit,
depender de una CLI de IA concreta.

## Forma de trabajar

- Pasos pequeños. Verificar `cargo fmt`, `cargo test`, `cargo clippy -p lolterm-core`.
- El GUI: no romper el contrato JSON (`snapshot`, `write`, `resize`, `run`, …)
  sin actualizar main y React juntos.
- No `unwrap()` indiscriminado en Rust.
- No comentarios en código salvo que se pidan.
- No commits automáticos. Mensajes: `feat: …`, `fix: …`.
