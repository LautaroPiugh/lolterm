# LoLTerm 

---

## 1. Qué es LoLTerm

LoLTerm es un **terminal workspace local-first**: un entorno gráfico de trabajo donde las terminales, los procesos CLI, los proyectos y las máquinas locales/remotas son ciudadanos de primera clase.

La terminal no es un panel auxiliar de LoLTerm. **La terminal es el runtime principal del producto.**

Definición técnica corta:

> **LoLTerm es un multiplexor gráfico de terminales y un workspace para herramientas CLI, respaldado por un core de PTYs escrito en Rust.**

Definición conceptual:

> **LoLTerm convierte la terminal en un workspace.**

Definición de producto a largo plazo:

> **LoLTerm es el entorno desde el que trabajo, no la herramienta con la que hago cada trabajo.**

LoLTerm organiza y ejecuta herramientas existentes en vez de reemplazarlas.

```text
LoLTerm no reemplaza nvim.
LoLTerm abre nvim.

LoLTerm no reemplaza Codex/Claude/OpenCode.
LoLTerm los abre y eventualmente les proporciona contexto.

LoLTerm no reemplaza Git.
LoLTerm conoce el repositorio y puede abrir lazygit u otra CLI.

LoLTerm no reemplaza SSH.
LoLTerm organiza máquinas y utiliza SSH.

LoLTerm no reemplaza tmux.
LoLTerm puede utilizar tmux, especialmente para persistencia remota.
```

---

## 2. Visión original que debe preservarse

La idea de LoLTerm no nació como “otra terminal bonita”. La visión es disponer de **un entorno personal de desarrollo instalable en cualquier máquina**, desde el cual sea posible trabajar con:

* terminales locales;
* proyectos y workspaces;
* herramientas CLI;
* editores como Neovim;
* Git y herramientas como lazygit;
* agentes/CLIs de IA como Codex, Claude Code u otros;
* contexto del proyecto;
* máquinas remotas mediante Tailscale + SSH + tmux;
* configuración portable;
* utilidades personales, eventualmente incluyendo música y otros módulos.

Representación conceptual:

```text
                 LoLTerm
                    │
        ┌───────────┼────────────┐
        │           │            │
     projects     machines     context
        │           │            │
        ↓           ↓            ↓
     terminals   SSH/tmux       AI CLIs
        │
 ┌──────┼───────────┐
 │      │           │
nvim  codex      lazygit
```

La evolución debe conservar esta visión sin intentar construir todas las capas al mismo tiempo.

---

## 3. Identidad: qué NO es LoLTerm

LoLTerm **no es**:

* un IDE tradicional;
* un editor de texto propio;
* un fork o wrapper de Vim/Neovim;
* un fork o wrapper de tmux;
* un runtime de agentes de IA;
* un chat de IA con una terminal pegada;
* un cliente Git completo;
* un file manager completo;
* un backend cloud obligatorio;
* un producto dependiente de una CLI de IA específica.

LoLTerm puede inspirarse en Vim/Neovim, tmux/Zellij y VS Code/Cursor en interacción y UX, pero debe conservar una arquitectura propia.

### Inspiraciones

**Vim/Neovim** aporta principalmente:

* enfoque keyboard-first;
* navegación rápida;
* comandos;
* keybindings configurables;
* modos cuando tengan sentido;
* composabilidad.

**tmux/Zellij** aporta principalmente:

* panes;
* splits;
* tabs/windows;
* sesiones;
* layouts;
* navegación entre terminales.

**VS Code/Cursor** aporta principalmente:

* chrome gráfico;
* sidebar;
* explorer;
* command palette;
* tabs visuales;
* experiencia de aplicación de escritorio.

La dirección es:

> **Vim-inspired + tmux-inspired + IDE-like GUI, pero LoLTerm-native.**

---

## 4. Principios fundamentales de producto

### 4.1. CLI-agnostic

LoLTerm es agnóstico respecto de lo que corre dentro de una terminal.

Para el core, estos procesos son conceptualmente equivalentes:

```text
bash
zsh
nvim
lazygit
btop
yazi
codex
claude
opencode
ssh
tmux
```

Todos son, en esencia:

```text
Process
  └── ejecutándose dentro de un PTY
```

No diseñar el runtime alrededor de Codex, Claude ni de ningún proveedor concreto.

### 4.2. Terminal-first

La unidad fundamental de ejecución es el proceso de terminal.

Un IDE tradicional suele pensar:

```text
Workspace
├── archivo.ts
├── archivo.rs
├── archivo.md
└── terminal
```

LoLTerm debe aproximarse más a:

```text
Workspace
├── process: nvim
├── process: codex
├── process: lazygit
├── process: dev server
└── process: ssh machine
```

Los archivos y el explorer son importantes, pero el runtime sigue siendo la terminal.

### 4.3. Local-first

LoLTerm debe poder funcionar plenamente sin una cuenta, una nube propia ni un backend HTTP central.

Por defecto:

```text
usuario
  ↓
LoLTerm
  ↓
config local
workspace local
PTYs locales
SSH a otras máquinas
```

La configuración portable podrá sincronizarse en el futuro mediante herramientas existentes como Git, dotfiles, chezmoi o Syncthing antes de considerar un servicio cloud propio.

### 4.4. Composable

Preferir combinar herramientas maduras antes que reimplementarlas.

Ejemplos:

```text
Git → git CLI / lazygit
Editor → nvim / helix / emacs / $EDITOR
Remote → Tailscale + SSH
Persistencia remota → tmux
File manager → yazi u otra CLI
AI → Codex / Claude / OpenCode / futuras CLIs
```

### 4.5. Portable

El diseño debe evitar acoplar el core al framework gráfico.

Prioridad inicial de plataforma:

1. Linux — plataforma principal de desarrollo actual.
2. macOS — soporte posterior.
3. Windows — soporte cuando el core y empaquetado estén suficientemente maduros.

No introducir dependencias de OS sin aislarlas y explicar el impacto.

### 4.6. GUI-first, no GUI-only architecture

La aplicación Desktop es el producto principal de LoLTerm.

Sin embargo, **no** se debe diseñar la arquitectura como si únicamente pudiera existir la GUI.

Dirección prevista:

```text
                 Desktop GUI
                      │
                      │
LoLTerm CLI ───── LoLTerm Core
                      │
                      │
                Workspace Runtime
                      │
                 PTY / SSH / OS
```

No revivir la antigua TUI como producto principal.

Una CLI `lolterm` futura sí es parte de la visión, pero funcionará como **interfaz de control del mismo sistema**, no como una segunda implementación independiente.

---

## 5. Arquitectura actual

### Stack

* **Rust, edition 2024** — `crates/core`.
* **Electron** — proceso Desktop y acceso a APIs del sistema operativo.
* **React + TypeScript** — interfaz gráfica.
* **Vite** — tooling/build del renderer.
* **xterm.js** — emulador de terminal VT dentro del DOM.
* **portable-pty** — creación y control de pseudoterminales reales.
* **Serde / JSON / TOML** — serialización de IPC, configuración y sesión.

### Conceptos

**PTY — Pseudo Terminal / pseudoterminal**

Abstracción del sistema operativo que hace que un proceso interactivo crea que está conectado a una terminal real.

```text
LoLTerm
   ↓
crea PTY
   ↓
spawn nvim / bash / ssh / codex
```

**IPC — Inter-Process Communication**

Comunicación entre procesos. En LoLTerm conecta la aplicación Electron con el sidecar Rust.

**Sidecar**

Proceso auxiliar ejecutado junto a la aplicación principal. Actualmente `lolterm-core` cumple esta función.

**Electron Main**

Proceso Node/Electron con acceso a APIs del sistema operativo. Gestiona ventanas, sidecar y puentes hacia el renderer.

**Renderer**

Interfaz React renderizada por Chromium.

**Preload**

Puente controlado entre Renderer y Main. Debe exponer una API pequeña y segura mediante `window.lolterm` o equivalente.

### Estructura actual

```text
lolterm/
├── AGENTS.md
├── Cargo.toml
├── Cargo.lock
├── README.md
├── crates/
│   └── core/
│       ├── Cargo.toml
│       └── src/
│           └── ...
└── apps/
    └── desktop/
        ├── package.json
        ├── electron/
        ├── src/
        ├── index.html
        ├── vite.config.ts
        └── figma-prompt.txt
```

### Flujo de terminal

```text
teclado/mouse
    ↓
React / xterm.js
    ↓
preload IPC
    ↓
Electron Main
    ↓
JSON line / contrato IPC
    ↓
lolterm-core (Rust)
    ↓
PTY write
    ↓
proceso
```

Lectura:

```text
proceso
   ↓
PTY read
   ↓
lolterm-core
   ↓
evento IPC
   ↓
xterm.write(...)
```

### Remoto

Dirección prevista:

```text
LoLTerm
   ↓
PTY
   ↓
ssh
   ↓
Tailscale / MagicDNS
   ↓
remote host
   ↓
tmux new-session -A -s <workspace/session>
```

Tailscale proporciona conectividad privada; SSH proporciona acceso; tmux mantiene sesiones remotas persistentes; LoLTerm organiza la experiencia.

---

## 6. Modelo conceptual objetivo

El modelo debe evolucionar gradualmente hacia:

```text
LoLTerm
└── Workspace
    ├── identity
    ├── root directory
    ├── target/machine
    ├── environment
    ├── tabs
    │   └── panes
    │       └── sessions
    │           └── PTY
    │               └── process
    ├── layout
    ├── startup commands
    ├── git metadata
    └── context metadata
```

No es obligatorio que los tipos actuales ya tengan exactamente estos nombres. Es la dirección conceptual.

### Workspace

Un workspace representa un entorno de trabajo recuperable, no sólo una colección temporal de terminales.

Ejemplo:

```text
Workspace: lolterm
Root: ~/dev/lolterm
Target: local
Git branch: main

┌──────────────────────┬──────────────┐
│                      │              │
│        nvim          │    codex     │
│                      │              │
├──────────────────────┴──────────────┤
│             shell                   │
└─────────────────────────────────────┘
```

En una fase posterior LoLTerm debe poder cerrar y reconstruir esta estructura con fidelidad razonable.

---

## 7. Command Registry: dirección arquitectónica

Las acciones de LoLTerm deberían tender a expresarse como comandos internos registrados, en vez de handlers dispersos por toda la UI.

Ejemplo conceptual:

```text
terminal.new
terminal.restart

pane.splitRight
pane.splitDown
pane.focusLeft
pane.focusRight
pane.focusUp
pane.focusDown
pane.zoom
pane.close

tab.new
tab.close
tab.rename
tab.reorder

workspace.create
workspace.open
workspace.save
workspace.close

machine.connect
process.run
file.open
theme.set
```

Un comando puede ser invocado eventualmente por:

```text
keyboard shortcut
command palette
UI button
LoLTerm CLI
plugin
AI agent
```

Todos deben converger sobre la misma lógica siempre que sea razonable.

No implementar un Command Registry gigante de una vez. Introducirlo de forma incremental cuando simplifique una feature real.

---

## 8. LoLTerm CLI: dirección futura

En una fase posterior debe existir un binario/control surface llamado aproximadamente:

```bash
lolterm
```

Ejemplos de UX objetivo:

```bash
lolterm .
lolterm ~/dev/api

lolterm workspace list
lolterm workspace open lolterm

lolterm run codex
lolterm run nvim

lolterm ssh chae
lolterm status
lolterm context
```

El CLI no debe duplicar el core.

Objetivo:

```text
                 ┌── Desktop GUI
                 │
lolterm-core  ←───┼── lolterm CLI
                 │
                 └── futuras integraciones
```

Antes de implementar esta capa, estabilizar los modelos de terminal, mux y workspace.

---

## 9. Context Layer

LoLTerm conoce y expone contexto estructurado del entorno de trabajo. Con el Desktop abierto, `lolterm context` habla con el mux (Unix socket de solo lectura). Si no hay instancia, lee `session.toml` y marca `"live": false`.

Ejemplo:

```json
{
  "live": true,
  "workspace": "lolterm",
  "cwd": "~/dev/lolterm",
  "machine": "local",
  "git": {
    "branch": "main"
  },
  "processes": [
    "nvim",
    "codex"
  ],
  "env": ["TERM", "EDITOR"]
}
```

```bash
lolterm context
lolterm workspace current
lolterm panes
lolterm processes
lolterm machines
```

La finalidad es que cualquier herramienta pueda consumir el contexto, especialmente agentes CLI.

```text
              LoLTerm Context
                    │
       ┌────────────┼────────────┐
       ↓            ↓            ↓
     Codex        Claude       OpenCode
```

No diseñar el contexto para un proveedor concreto.

---

## 10. IA dentro de LoLTerm

La IA es importante para la visión, pero no debe redefinir el producto.

Regla:

> **LoLTerm organiza agentes; LoLTerm no implementa la lógica interna de los agentes.**

Por defecto un agente es simplemente una CLI dentro de un PTY:

```text
Pane
└── PTY
    └── codex
```

Futuras comodidades válidas:

* templates para abrir agentes (paleta `/codex`, picker `+`);
* contexto del workspace (`LOLTERM_CONTEXT`, `LOLTERM_ROOT`, `LOLTERM_WORKSPACE`);
* worktrees por agente (`LOLTERM_WORKTREE`, `~/.local/share/lolterm/worktrees/`);
* indicadores de estado y aviso al cerrar;
* historial corto de lanzamientos;

Evitar construir prematuramente:

* chat IA propio;
* runtime de agentes;
* ACP propio;
* sistema cerrado dependiente de una sola IA.

---

## 11. Roadmap oficial

El roadmap expresa **eras funcionales**, no una promesa rígida de que cada `MINOR` sólo contendrá exactamente esas features.

Las versiones reales las determina Semantic Versioning y el contenido de las releases.

### v0.1.x — Terminal Foundation

Objetivo:

> Poder usar una terminal dentro de LoLTerm durante todo el día sin sentir que el emulador/PTY es experimental.

Prioridades:

* lifecycle correcto de PTYs;
* entrada de teclado;
* focus estable;
* resize correcto;
* xterm fit;
* clipboard;
* scroll;
* mouse events;
* Unicode;
* TrueColor;
* ANSI/VT compatibility;
* alternate screen;
* exit handling;
* cleanup de procesos;
* splits estables;
* tab UX básica;
* estabilidad de IPC;
* tests del core.

Programas de prueba obligatorios/recomendados:

```text
nvim
lazygit
btop
fzf
yazi
codex
claude
ssh
tmux
```

`ls` funcionando no es una validación suficiente del emulador.

### v0.2.x — Multiplexer

Objetivo: que LoLTerm sea un multiplexor completo y cómodo.

Features objetivo:

* split horizontal/vertical;
* drag resize;
* focus navigation;
* swap/move panes;
* pane zoom;
* close/restart;
* tab rename;
* tab reorder;
* tab duplicate cuando tenga sentido;
* serialización de layouts;
* presets;
* restauración más fiel;
* Command Registry incremental;
* keybindings configurables.

La UX puede inspirarse en Vim/tmux, pero los defaults deben ser coherentes con LoLTerm.

### v0.3.x — Workspaces

Objetivo: convertir colecciones de terminales en entornos persistentes.

Features objetivo:

* múltiples workspaces;
* root directory;
* layout por workspace;
* tabs/panes persistentes;
* variables de entorno;
* startup commands;
* open/save/restore;
* project metadata;
* navegación entre workspaces.

### v0.4.x — Remote

Objetivo: que local y remoto se sientan como variantes del mismo concepto.

Features objetivo:

* machine registry;
* SSH mejorado;
* Tailscale/MagicDNS;
* usuario configurable;
* recientes;
* conexión visual desde sidebar/palette;
* integración opcional con tmux remoto;
* recuperación de sesión remota.

Modelo:

```text
local  → PTY → shell
remote → PTY → ssh → Tailscale → tmux/shell
```

### v0.5.x — LoLTerm CLI + distribución inicial

Objetivo: exponer el sistema por CLI y comenzar a distribuir builds utilizables.

Features objetivo:

```bash
lolterm .
lolterm workspace list
lolterm workspace open <name>
lolterm run <command>
lolterm ssh <machine>
lolterm status
```

También:

* empaquetado Desktop reproducible;
* Linux artifact inicial;
* pipeline de release que adjunte artefactos;
* macOS/Windows sólo cuando estén técnicamente maduros.

### v0.6.x — Context Layer

Objetivo: que LoLTerm comprenda el entorno actual y pueda exponerlo estructuradamente.

Contexto posible:

* workspace;
* cwd;
* repo;
* branch;
* target/machine;
* procesos;
* panes;
* archivos recientes cuando sea seguro/útil;
* variables de entorno seleccionadas.

Evitar filtrar secretos en el contexto.

### v0.7.x — AI Environment

Objetivo: hacer de LoLTerm un excelente host para herramientas de IA sin convertirse en un runtime propietario de agentes.

Features posibles:

* launchers de agentes;
* context providers;
* variables de entorno/context files;
* worktrees;
* session status;
* notificaciones;
* comandos consumibles por agentes.

### v0.8.x — Extensibility

Objetivo: abrir capacidades estables del producto.

Primera entrega (TOML local, sin JS/WASM):

* custom commands (`ext.<slug>`);
* themes (`themes/*.toml`);
* hooks (`workspace.open`);
* context providers (`context.toml` → `extra`);
* status items (`status.toml`);
* packs `extensions/<nombre>/extension.toml`.

No hay paneles React custom ni plugins remotos. No definir una API de plugins JS antes de estabilizar estas abstracciones.

### v0.9.x — Stabilization & Distribution

Objetivo: preparar 1.0.

Prioridades:

* performance;
* memoria;
* startup time;
* crashes;
* PTY leaks;
* IPC robustness;
* SSH reliability;
* session restoration;
* keyboard edge cases;
* security;
* packaging;
* installers;
* code signing donde corresponda;
* auto-update;
* documentación;
* compatibilidad de plataformas declaradas.

### v1.0.0 — Personal Developer Environment

Criterio conceptual:

> Instalar LoLTerm en una máquina nueva debe permitir utilizarlo como entorno principal de desarrollo sin depender de abrir otra terminal para el flujo cotidiano.

1.0 no significa “todas las ideas posibles implementadas”. Significa que el núcleo del producto es estable, instalable, mantenible y coherente con su visión.

---

## 12. Qué NO priorizar todavía

Evitar feature creep.

No introducir prematuramente:

```text
editor propio
Monaco como centro del producto
LSP propio/integrado como prioridad
chat IA propio
runtime de agentes
cliente Git completo
file manager complejo
Spotify/music clone
daemon distribuido
cloud propio
plugin API inestable
Windows a cualquier costo
Tauri migration sin necesidad medida
```

Música y utilidades personales siguen siendo compatibles con la visión, pero pertenecen a una capa posterior del **Personal Environment**, no al Terminal Foundation.

Orden mental recomendado:

```text
terminal
   ↓
multiplexer
   ↓
workspace
   ↓
remote
   ↓
CLI
   ↓
context
   ↓
AI ecosystem
   ↓
extensibility
   ↓
personal environment
```

---

## 13. Estado del repositorio al 2026-08-17

El repositorio actual ya contiene una base funcional:

* workspace Rust;
* `crates/core`;
* sidecar `lolterm-core`;
* Electron + React + TypeScript;
* xterm.js;
* PTY local real;
* tabs;
* splits;
* command palette;
* explorer;
* overlay Git básico;
* sesión básica;
* contexto en vivo vía Unix socket (`lolterm context`);
* cada PTY recibe `LOLTERM_CONTEXT` (JSON en el runtime dir);
* agentes en git worktree + status/historial;
* extensiones TOML (comandos, hooks, temas, status, context.extra);
* SSH/Tailscale en desarrollo;
* interfaz visual Sage/mint inspirada en la referencia inicial.

Versión global al alinear este documento con el código (era Extensibility, `v0.8.x`):

```text
Cargo workspace:          0.8.0
apps/desktop/package.json 0.8.0
lolterm / lolterm-core    0.8.0
```

Una sola versión de producto. No volver a divergir a mano salvo un release explícito.

---

## 14. Versionado oficial

LoLTerm utiliza **Semantic Versioning (SemVer)**:

```text
MAJOR.MINOR.PATCH
```

Ejemplo:

```text
0.3.7
│ │ │
│ │ └── PATCH
│ └──── MINOR
└────── MAJOR
```

### PATCH

Correcciones compatibles.

```text
0.3.1 → 0.3.2
```

Ejemplo:

```text
fix: correct terminal resize after split
```

### MINOR

Nueva funcionalidad compatible o salto funcional relevante.

```text
0.3.2 → 0.4.0
```

Ejemplo:

```text
feat: add remote workspace targets
```

### MAJOR

Cambio incompatible importante en una versión estable.

```text
1.4.2 → 2.0.0
```

Ejemplo conceptual:

```text
feat!: replace workspace configuration format
```

### Antes de 1.0

Durante `0.x`, la API y arquitectura aún pueden evolucionar. Aun así, evitar cambios incompatibles gratuitos. Explicar migraciones y preservar estado/config siempre que sea razonable.

---

## 15. Una versión global

No manejar manualmente versiones independientes para cada componente del producto salvo que una necesidad futura lo justifique explícitamente.

Objetivo:

```text
LoLTerm 0.8.0
├── Cargo workspace   0.8.0
├── lolterm-core      0.8.0
└── desktop package   0.8.0
```

La automatización de release debe mantener sincronizados los archivos pertinentes, incluyendo al menos:

* `Cargo.toml` del workspace;
* `apps/desktop/package.json`;
* cualquier metadata de packaging que luego incorpore una versión explícita.

No actualizar versiones manualmente en cada feature/fix si el pipeline de release ya lo gestiona.

---

## 16. Commits: Conventional Commits

Usar **Conventional Commits** para que cambios, changelog y versionado sean legibles por humanos y automatizables.

Formatos principales:

```text
feat: add pane zoom
fix: restore terminal focus after tab switch
docs: document ssh workflow
refactor: simplify pty lifecycle
test: add session restore coverage
chore: update dependencies
build: configure desktop packaging
ci: add rust and desktop checks
perf: reduce terminal render overhead
```

Scopes son opcionales cuando agregan claridad:

```text
feat(mux): add pane zoom
fix(pty): close child process on pane exit
fix(desktop): restore xterm focus after tab switch
```

Breaking change:

```text
feat(workspace)!: replace workspace schema
```

No usar mensajes vacíos o ambiguos como:

```text
changes
update
fix stuff
wip final
misc
```

### Importante para agentes

**No crear commits automáticamente salvo que el usuario lo pida explícitamente.**

Cuando el usuario sí solicite un commit, proponer/usar un mensaje Conventional Commit coherente con el cambio realizado.

---

## 17. Commit ≠ Release

Una modificación de código no debe provocar necesariamente una publicación.

```text
modification
   ↓
commit
   ↓
CI
   ↓
merge(s)
   ↓
release candidate PR
   ↓
release
```

Las versiones se publican cuando hay un conjunto coherente de cambios listo para distribuir.

No generar un release por un typo, ajuste de padding o commit trivial salvo que el usuario defina explícitamente otra política.

---

## 18. Release automation objetivo

La estrategia preferida es **Release Please + GitHub Actions**, salvo que una futura restricción técnica justifique cambiarla.

Flujo esperado:

```text
Conventional Commits
        ↓
GitHub Actions CI
        ↓
main
        ↓
Release Please
        ↓
Release PR
├── versión propuesta
├── CHANGELOG.md
└── release notes
        ↓
merge del Release PR
        ↓
tag vX.Y.Z
        ↓
GitHub Release
        ↓
build artifacts
```

### Release Please

Debe encargarse gradualmente de:

* analizar Conventional Commits;
* calcular la próxima versión;
* mantener `CHANGELOG.md`;
* actualizar archivos de versión;
* crear el Release PR;
* crear tag/release después del merge correspondiente.

El agente no debe inventar números de versión manuales cuando la automatización pueda calcularlos.

### Git tags

Formato:

```text
v0.1.0
v0.2.0
v0.5.3
v1.0.0
```

### CHANGELOG.md

Debe ser generado/mantenido principalmente por el sistema de release.

No duplicar manualmente cada commit en el changelog si Release Please ya es la fuente del flujo.

---

## 19. CI — Continuous Integration

**CI** significa *Continuous Integration* o integración continua.

Cada Pull Request y los pushes relevantes deben validar el proyecto antes de considerarlo listo.

Objetivo mínimo del pipeline:

### Rust

```bash
cargo fmt --check
cargo clippy -p lolterm-core --all-targets --all-features -- -D warnings
cargo test -p lolterm-core
```

Ajustar `--all-features` si aparecen features incompatibles entre sí; no copiar flags ciegamente.

### Desktop

Desde `apps/desktop`:

```bash
npm ci
npm run build
```

Agregar `typecheck`, lint o tests cuando existan scripts reales para ellos.

No inventar comandos inexistentes. Si falta un script valioso, explicar primero y luego agregarlo deliberadamente.

### Regla de release

No publicar una release cuyo commit objetivo no pase CI.

---

## 20. Packaging y auto-update

No es prioridad de `v0.1`, pero debe mantenerse una dirección clara.

Fases:

```text
source/dev
   ↓
packaging
   ↓
release artifacts
   ↓
installers
   ↓
code signing
   ↓
auto-update
```

Para Desktop, evaluar Electron Forge cuando llegue la fase de distribución, salvo que el stack cambie justificadamente.

Artefactos objetivo futuros:

```text
Linux   → AppImage / .deb u otro formato definido
macOS   → .dmg / signed app
Windows → .exe/.msi según tooling elegido
```

No implementar auto-update antes de resolver correctamente:

* canales de release;
* firmas;
* origen confiable de artefactos;
* rollback/fallos;
* comportamiento por plataforma.

Auto-update corresponde aproximadamente a `v0.9.x`.

---

## 21. Configuración y estado

Dirección preferida:

```text
~/.config/lolterm/
├── config.toml
├── keybindings.toml
├── workspaces.toml
├── commands.toml
├── hooks.toml
├── status.toml
├── context.toml
├── themes/
│   └── nord.toml
└── extensions/
    └── <nombre>/
        └── extension.toml
```

Separar conceptualmente:

**Portable config**

* temas;
* keybindings;
* workspace definitions sin secretos;
* preferencias.

**Machine-local state**

* PID/process state;
* `$XDG_RUNTIME_DIR/lolterm/mux.sock` (consulta de contexto; no es config portable);
* `$XDG_RUNTIME_DIR/lolterm/context.json` (misma foto; los PTYs la ven en `LOLTERM_CONTEXT`);
* `$XDG_DATA_HOME/lolterm/worktrees/` e `agent-sessions.jsonl` (agentes; no portable);
* paths específicos cuando no sean portables;
* geometría de ventanas;
* cache;
* datos efímeros.

Nunca guardar secretos en archivos sincronizables sin un diseño explícito de seguridad.

---

## 22. Seguridad

Aunque LoLTerm sea local-first, manejará comandos, shells, SSH y potencialmente secretos.

Reglas:

* no loggear passwords, tokens, claves privadas ni variables sensibles;
* no enviar contexto a servicios externos implícitamente;
* no ejecutar comandos destructivos sin que la acción del usuario sea clara;
* mantener el preload de Electron con la superficie mínima necesaria;
* no habilitar APIs Node arbitrarias dentro del renderer;
* validar IPC en los límites entre procesos;
* tratar input remoto como no confiable;
* no guardar credenciales SSH en texto plano;
* preferir agent/keychain/SSH existente del sistema cuando corresponda;
* explicar implicaciones de seguridad al introducir una nueva capacidad privilegiada.

---

## 23. Regla fundamental de enseñanza

El autor está construyendo LoLTerm mientras aprende Rust y el stack Desktop.

**No escribir código sin explicar.**

Esto aplica especialmente a:

* Rust;
* Electron;
* React;
* TypeScript;
* xterm.js;
* PTYs;
* IPC;
* concurrencia;
* networking/SSH;
* packaging;
* GitHub Actions;
* release automation.

### Antes de modificar código, explicar

1. Qué problema se va a resolver.
2. Qué concepto técnico interviene.
3. Qué significa cada sigla o término técnico nuevo.
4. Por qué se elige la solución propuesta.
5. Qué alternativas razonables existen.
6. Qué archivos se planea modificar.
7. Cómo encaja el cambio en la arquitectura general.

La explicación debe ser concreta y proporcional. No convertir una corrección de tres líneas en una clase de veinte páginas.

### Después de implementar, explicar

1. Qué cambió.
2. Cómo funciona.
3. Qué partes del código son importantes.
4. Qué sintaxis nueva de Rust/TypeScript apareció.
5. Cómo probarlo con comandos concretos.
6. Qué errores o edge cases podrían aparecer.
7. Qué concepto conviene retener de lo aprendido.

### Terminología

No asumir que abreviaciones o jerga son conocidas la primera vez que aparecen en un contexto importante.

Ejemplos a explicar cuando corresponda:

* PTY;
* TTY;
* VT;
* ANSI;
* IPC;
* RPC;
* JSON-RPC;
* sidecar;
* renderer;
* preload;
* event loop;
* thread;
* channel;
* ownership;
* borrowing;
* lifetime;
* mutex;
* async;
* CI;
* CD;
* SemVer;
* Conventional Commits.

Cuando ayude, comparar conceptos de Rust con JavaScript/PHP, pero sin deformar el modelo técnico.

No repetir definiciones completas que ya se explicaron salvo que el concepto tenga un uso nuevo o exista riesgo de confusión.

---

## 24. Forma de trabajar de los agentes

### 24.1. Pasos pequeños

Preferir cambios acotados y verificables.

No implementar una fase completa del roadmap de una sola vez.

Ejemplo correcto:

```text
1. resolver xterm focus
2. probar nvim
3. resolver resize
4. probar btop
5. recién después drag split
```

Ejemplo incorrecto:

```text
“Implementé terminal, workspace, remote, plugins e IA en un único cambio.”
```

### 24.2. Leer antes de editar

Antes de modificar una zona:

* leer los archivos relevantes;
* identificar la fuente de verdad;
* seguir patrones existentes cuando sean razonables;
* no asumir contratos de IPC ni tipos sin inspeccionarlos.

### 24.3. Evitar refactors gratuitos

No mezclar una feature/fix con una reescritura amplia no solicitada.

Refactorizar sólo cuando:

* sea necesario para implementar correctamente el cambio;
* reduzca complejidad real;
* exista una razón explicable;
* pueda verificarse sin aumentar innecesariamente el alcance.

### 24.4. No ocultar problemas

No introducir hacks silenciosos sólo para hacer pasar un caso feliz.

Si existe deuda técnica relevante, explicarla y delimitarla.

### 24.5. No agregar dependencias sin justificación

Antes de incorporar una dependencia:

* explicar qué resuelve;
* por qué la stdlib/stack actual no basta;
* costo de mantenimiento;
* superficie de seguridad;
* impacto de bundle/build;
* alternativas consideradas.

### 24.6. Mantener contratos sincronizados

Si cambia el contrato entre:

```text
Rust core
↕
Electron main/preload
↕
React renderer
```

actualizar todas las capas afectadas en el mismo cambio y verificar compatibilidad.

No romper mensajes IPC (`snapshot`, `write`, `resize`, `run`, etc.) silenciosamente.

---

## 25. Reglas Rust

* Preferir errores propagados con contexto en lugar de `unwrap()` indiscriminado.
* Mantener ownership de PTYs/procesos claro.
* Evitar leaks de threads y child processes.
* No bloquear innecesariamente el hilo que sirve IPC.
* Preferir canales/hilos existentes antes de introducir Tokio sólo por costumbre.
* Tokio puede introducirse si existe una necesidad real y explicada de async I/O/concurrencia que justifique el cambio arquitectónico.
* Evitar `unsafe` salvo que sea estrictamente necesario; documentar por qué es sound.
* Agregar tests para lógica pura y bugs reproducibles cuando sea razonable.
* Mantener `cargo fmt` y `cargo clippy` limpios.

### Core como frontera

El core debe concentrar capacidades de sistema reutilizables:

* PTY lifecycle;
* process spawning;
* mux/session state;
* SSH/remoto cuando corresponda;
* filesystem/Git metadata limitada cuando sea parte del producto;
* persistencia estructurada.

Evitar meter lógica puramente visual de React dentro del core.

---

## 26. Reglas Electron / React / TypeScript

* Mantener separación Main / Preload / Renderer.
* Exponer desde preload sólo operaciones necesarias.
* Evitar habilitar Node integration global en el renderer.
* Mantener xterm lifecycle sincronizado con pane/session lifecycle.
* Disponer listeners al desmontar componentes.
* Evitar listeners duplicados después de re-renders.
* Resize debe coordinar DOM size → xterm fit → PTY dimensions.
* El pane activo debe recibir focus de forma predecible.
* No esconder estado importante solamente dentro de componentes si pertenece al modelo de workspace/session.
* No convertir `App.tsx` en un monolito; extraer componentes/estado cuando exista una frontera real.
* No abstraer prematuramente componentes de una sola línea sólo “por arquitectura”.

---

## 27. Terminal correctness: criterios de calidad

Una terminal interactiva es más exigente que imprimir stdout.

Cada cambio importante del emulador/mux debe considerar:

* dimensiones `rows × cols`;
* SIGWINCH/resize equivalente;
* alternate screen;
* raw mode del programa hijo;
* control sequences;
* mouse reporting;
* bracketed paste;
* Ctrl/Alt combinations;
* Unicode y wide characters;
* TrueColor;
* scrollback;
* focus;
* clipboard;
* process exit;
* terminal cleanup.

Tests manuales recomendados:

```bash
nvim
lazygit
btop
fzf
yazi
ssh <host>
tmux
codex
```

Probar resize mientras esas TUIs están abiertas.

---

## 28. UX y diseño

LoLTerm debe sentirse como una herramienta profesional de teclado, pero no hostil al mouse.

Principios:

* keyboard-first, mouse-capable;
* chrome sobrio;
* terminal con máxima prioridad visual;
* command palette rápida;
* pocos overlays persistentes;
* feedback de estado claro;
* tabs/panes reconocibles;
* remote/local distinguibles sin saturar la interfaz;
* no cubrir el prompt con overlays innecesarios;
* notices temporales con TTL/dismiss cuando corresponda.

La referencia Sage/mint del proyecto es una inspiración visual, no una obligación de copiar píxel por píxel.

---

## 29. Nomenclatura del producto

Mantener **LoLTerm** como nombre mientras la terminal continúe siendo el fundamento del sistema.

No renombrar el proyecto simplemente porque incorpore:

* workspaces;
* IA;
* remote machines;
* context;
* plugins;
* música;
* utilidades.

El nombre debe revisarse sólo si la terminal deja de ser el elemento estructural principal del producto.

Capitalización preferida en prosa:

```text
LoLTerm
```

Binarios/paths pueden usar lowercase:

```text
lolterm
lolterm-core
```

---

## 30. Definición de “hecho” para una modificación

Una tarea no está terminada sólo porque “compila”.

Para considerarla hecha, cuando aplique:

1. el problema está realmente resuelto;
2. no rompe la arquitectura definida;
3. el código relevante fue formateado;
4. tests existentes pasan;
5. Clippy/build/type checks correspondientes pasan;
6. se hizo al menos una prueba manual del flujo interactivo si corresponde;
7. no se introdujeron dependencias injustificadas;
8. se documentó cualquier cambio de contrato/config relevante;
9. se explicaron al autor el concepto y el cambio;
10. si el usuario pide commit, el mensaje sigue Conventional Commits.

---

## 31. Comandos de verificación actuales

Desde la raíz:

```bash
cargo fmt --check
cargo clippy -p lolterm-core --all-targets -- -D warnings
cargo test -p lolterm-core
```

Desktop:

```bash
cd apps/desktop
npm ci
npm run build
npm run dev
```

En desarrollo local puede utilizarse `npm install` cuando se están modificando dependencias. En CI preferir `npm ci` con lockfile consistente.

En Linux, el flujo Electron actual puede requerir `--no-sandbox` por el helper SUID `chrome-sandbox` cuando la instalación de usuario no conserva permisos root/4755. Tratar esto como una particularidad de desarrollo, no como el modelo de seguridad final del producto.

---

## 32. Próximo foco inmediato

LoLTerm está en **`v0.8.x` (Extensibility)**. No construir un runtime de plugins JS. No agrandar el catálogo de la CLI. No empezar v0.9 (empaquetado/auto-update) salvo instrucción explícita.

Orden recomendado:

### A. Extensiones TOML (huecos de v0.8)

Probar `commands.toml`, un tema custom, un hook `workspace.open` y un `status.toml`. Los ids de comando deben ser `ext.<slug>`. En Desktop, `/commands` (o el engranaje) edita `commands.toml` y `keybindings.toml`; el archivo sigue siendo la fuente de verdad.

### B. Multiplexer cómodo (huecos de v0.2)

Pruebas diarias con nvim/lazygit/btop/fzf/yazi/ssh/tmux; resize/focus si fallan.

### C. Workspaces (v0.3, ya hay base)

Restauración fiel, startup, navegación entre workspaces. No reimplementar el explorer.

---

## 33. Regla de decisión para nuevas features

Antes de aceptar una feature, preguntarse:

> **¿Esto fortalece LoLTerm como entorno desde el que trabajo, o intenta reemplazar innecesariamente otra herramienta?**

Preferir:

```text
abrir / organizar / contextualizar / conectar / persistir
```

antes que:

```text
reimplementar / encerrar / duplicar / acoplar
```

Ejemplos:

**Sí encaja**

* abrir `$EDITOR` en un pane;
* guardar un layout;
* abrir Codex con contexto del workspace;
* conectar a una máquina Tailscale;
* restaurar tmux remoto;
* exponer `lolterm context`;
* registrar comandos.

**Probablemente no ahora**

* implementar un editor completo;
* crear un protocolo de IA propio;
* reemplazar Git;
* construir un navegador de archivos avanzado antes del mux;
* implementar streaming de música desde cero.

---

## 34. Norte arquitectónico final

La evolución buscada puede resumirse así:

```text
┌───────────────────────────────────────────────┐
│        PERSONAL DEVELOPER ENVIRONMENT         │
│ context · AI · utilities · portable config   │
├───────────────────────────────────────────────┤
│                  WORKSPACE                    │
│ projects · layouts · sessions · machines     │
├───────────────────────────────────────────────┤
│                 MULTIPLEXER                   │
│ panes · tabs · navigation · persistence      │
├───────────────────────────────────────────────┤
│              TERMINAL PLATFORM                │
│ PTY · process · ANSI · resize · input · SSH  │
├───────────────────────────────────────────────┤
│                  RUST CORE                    │
│ OS primitives · state · reusable runtime     │
└───────────────────────────────────────────────┘
          ↑                         ↑
      Desktop GUI              lolterm CLI
```

La capa superior no debe comprometer la solidez de las inferiores.

---

## 35. Resumen ejecutivo para cualquier agente nuevo

Si sólo se leen unas pocas reglas, deben ser estas:

1. **LoLTerm es un terminal workspace, no un IDE ni un runtime de IA.**
2. **La terminal/proceso es la unidad fundamental del sistema.**
3. **Todo lo que corre dentro del PTY debe ser tratado de forma CLI-agnostic.**
4. **La aplicación es local-first y composable.**
5. **Desktop es la interfaz principal; el core Rust debe seguir siendo reutilizable.**
6. **No reimplementar nvim, Git, SSH, tmux o agentes de IA. Integrarlos.**
7. **Primero terminal sólida; después mux; workspace; remote; CLI; context; AI; extensibility.**
8. **Explicar antes y después de programar porque el autor está aprendiendo el stack.**
9. **Cambios pequeños, verificables y sin refactors gratuitos.**
10. **Usar SemVer + Conventional Commits + CI + Release Please para releases.**
11. **No crear commits ni releases automáticamente salvo instrucción explícita del usuario; la infraestructura puede preparar Release PRs automáticamente.**
12. **La meta de 1.0 es que LoLTerm pueda ser el entorno diario de desarrollo del autor.**

---

## 36. Frase guía

Cuando haya dudas de producto o arquitectura, volver a esta frase:

> **LoLTerm es el entorno desde el que trabajo, no la herramienta con la que hago cada trabajo.**
