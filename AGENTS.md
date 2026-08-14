# LolTerm

Workspace de terminal personal, portátil y extensible. Inspirado en tmux, Zellij y
multiplexers modernos, pero orientado a integrar en una misma experiencia: terminales
locales, múltiples panes y tabs, workspaces por proyecto, cualquier CLI dentro de PTYs,
CLIs de IA (Codex, Claude Code, OpenCode, Gemini), contexto automático del proyecto,
SSH, Tailscale, sesiones remotas persistentes (tmux inicialmente), herramientas Git y,
más adelante, música.

## Regla fundamental de trabajo

No escribir código sin explicar. Cada cambio debe ser explicativo y educativo, porque el
autor está aprendiendo Rust y arquitectura de aplicaciones de terminal mientras construye.

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
4. Sintaxis de Rust nueva que apareció.
5. Cómo probarlo (comandos concretos).
6. Qué errores podrían aparecer.
7. Qué aprendimos.

No dar por sentada la terminología. Explicar brevemente términos como PTY, TUI, async,
runtime, trait, ownership, borrowing, lifetime, channel, thread, process, IPC, event
loop, renderer, buffer, mutex, daemon. No repetir explicaciones ya dadas salvo que
aparezca un uso nuevo. Hacer comparaciones con PHP, JavaScript/TypeScript cuando ayude.

## Filosofía arquitectónica

1. **CLI agnostic**: LolTerm no depende de ninguna CLI específica. Un pane ejecuta
   cualquier programa (`bash`, `zsh`, `fish`, `nvim`, `vim`, `lazygit`, `btop`, `codex`,
   `claude`, `opencode`, `gemini`, `ssh`, `tmux`, etc.). Abstracción principal:

   ```text
   LolTerm → PTY → Proceso CLI
   ```

2. **Local-first**: funciona localmente sin servidor central, cuenta, nube, BD remota ni
   backend HTTP. Las capacidades remotas vendrán después.

3. **Composable**: orquestar herramientas existentes (lazygit para Git, codex/claude/
   opencode/gemini para IA, ssh para remoto, tmux para persistencia remota, Tailscale con
   sus propias herramientas). No reimplementar lo que ya existe.

4. **Portable**: Linux y macOS primero; Windows se evaluará después.

## Stack tecnológico

- **Rust** (edition 2024)
- **Ratatui**: framework para interfaces de terminal.
- **TUI**: Terminal User Interface, interfaz visual dentro de una terminal.
- **portable-pty**: librería para crear pseudoterminales.
- **tui-term**: renderiza terminales dentro de Ratatui (usa `vt100` como parser ANSI).
- **Tokio**: runtime asíncrono para Rust.
- **color-eyre**: manejo y presentación amigable de errores.
- **Serde** / **TOML**: serialización y configuración (futuro).

Regla: antes de agregar una dependencia, explicar por qué es necesaria. No agregar por
comodidad si se puede resolver razonablemente con las existentes.

## Hitos

### LOL-001 — Interactive PTY (en curso)

`cargo run` abre una TUI con una terminal interactiva real. Flujo:

```text
LolTerm → Ratatui dibuja la interfaz → PTY → shell del usuario
```

Dentro deben funcionar `ls`, `pwd`, `echo hola`, `vim`, `nvim`, `htop`, `lazygit`, etc.,
como en una terminal normal.

**Criterios de aceptación**: inicia correctamente; entra en pantalla apropiada para TUI;
crea un PTY; inicia el shell del usuario; recibe output; lo renderiza; envía teclas
(Enter, Backspace y comunes); apps interactivas funcionan razonablemente; recibe resize;
cierra de forma controlada; restaura la terminal original al salir; restaura ante
error/panic; código organizado para evolucionar a múltiples panes.

### Progresión recomendada

1. verificar proyecto y dependencias
2. crear una TUI mínima
3. entrar y salir del alternate screen
4. crear un PTY
5. lanzar el shell
6. leer output del PTY
7. renderizar output
8. enviar teclado al PTY
9. manejar resize
10. probar programas interactivos
11. mejorar manejo de errores y cleanup

## Arquitectura

Estructura deseada (guía, no obligación; no construir componentes que aún no se
necesitan):

```text
src/
├── main.rs     # arranque de aplicación
├── app.rs      # estado general
├── tui.rs      # configuración y renderizado de Ratatui
├── event.rs    # teclado, resize y otros eventos
└── terminal.rs # PTY y proceso del shell
```

### Event loop (dirección futura)

```text
teclado + PTY → EVENT LOOP → App State → Render
```

### Alternate screen

Segunda pantalla temporal del terminal. Apps como vim/less/htop/tmux la usan para ocupar
toda la pantalla y devolver la terminal intacta. LolTerm la usa.

## Visión futura (no implementar todavía)

```text
LolTerm
├── Workspaces ├── Tabs ├── Panes ├── Projects ├── Context
├── AI launcher (Codex, Claude, OpenCode, Gemini)
├── SSH ├── Tailscale ├── Remote tmux ├── Git tools └── Music
```

Para remoto, inicialmente: `LolTerm → PTY → ssh → Tailscale → máquina remota → tmux`,
antes de desarrollar un daemon o protocolo propio.

## Forma de trabajar

- Avanzar iterativamente, por pasos pequeños.
- No saltar a una implementación enorme.
- Verificar cada paso con `cargo fmt`, `cargo check`, `cargo clippy`.
- No usar `unwrap()` indiscriminadamente; si se usa, explicar por qué la invariancia lo
  hace razonable. Preferir errores manejables y código legible.

## Calidad

`cargo fmt`, `cargo check`, `cargo clippy` sin errores relevantes. No silenciar warnings
innecesariamente. No agregar comentarios al código salvo que se pidan explícitamente.

## Git

- Revisar estado del repo (`git status`, `git diff`, `git log`).
- No hacer commits automáticamente; solo sugerir mensaje de commit al terminar una unidad
  lógica de trabajo.
- Estilo de mensajes: `feat: add interactive pty`, `fix: restore terminal on panic`,
  `refactor: separate terminal state`.

## Estado actual

- Repo sin commits todavía. Dependencias ya declaradas en `Cargo.toml`.
- LOL-001, incremento 1 completado: TUI mínima con entrada/salida limpia del alternate
  screen.
  - `src/main.rs`: arranque + `color_eyre::install()` + `tui::init()`/`tui::restore()`.
  - `src/tui.rs`: `init()` y `restore()` (wrappers de `ratatui::init`/`restore`).
  - `src/app.rs`: `App { running }` con `run()` (event loop), `draw()` y `handle_events()`.
- Próximo paso: crear el PTY y lanzar el shell (pasos 4-5), en un nuevo `src/terminal.rs`
  usando `portable_pty::native_pty_system()`, `openpty(...)` y
  `CommandBuilder::new_default_prog()`.
