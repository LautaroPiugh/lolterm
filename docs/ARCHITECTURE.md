# Arquitectura

LoLTerm es un multiplexor gráfico de terminales con core Rust y desktop Electron/React.

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

Lectura en sentido inverso:

```text
proceso → PTY → lolterm-core → evento IPC → xterm.write(...)
```

## Piezas

| Pieza | Ruta | Responsabilidad |
| --- | --- | --- |
| Core Rust | `crates/core` | PTYs, procesos, mux, sesiones, workspaces, SSH, contexto, filesystem/Git limitado |
| Binario sidecar | `lolterm-core` | proceso que Electron arranca y controla |
| CLI | `lolterm` | superficie de control: abrir workspace, run, ssh, context, panes, processes |
| Electron main | `apps/desktop/electron/main.mjs` | ventana, sidecar, IPC, updater, APIs de sistema |
| Preload | `apps/desktop/electron/preload.cjs` | puente seguro entre renderer y main |
| Renderer | `apps/desktop/src` | React UI, xterm.js, panes, tabs, overlays, settings |

## PTY

PTY significa pseudo terminal. Es la abstracción del sistema operativo que hace que un proceso interactivo crea que está conectado a una terminal real. Esto importa para `nvim`, `lazygit`, `ssh`, `tmux`, `fzf`, mouse reporting, resize y alternate screen.

## IPC

IPC significa comunicación entre procesos. LoLTerm usa una frontera clara:

```text
Renderer ⇄ preload ⇄ Electron main ⇄ lolterm-core
```

El renderer no debe recibir acceso Node libre. El preload debe exponer sólo operaciones necesarias.

## Mux y workspace

El mux administra tabs, panes y sesiones. Un workspace agrupa raíz de proyecto, layout, procesos y contexto. La dirección del modelo es:

```text
Workspace
├── root
├── machine/target
├── tabs
│   └── panes
│       └── PTY/process
├── layout
├── startup commands
└── context metadata
```

## Configuración y estado

- Config portable: `~/.config/lolterm/`.
- Runtime vivo: `$XDG_RUNTIME_DIR/lolterm/`.
- Datos locales: `$XDG_DATA_HOME/lolterm/`.
- Build outputs: `target/`, `apps/desktop/dist/`, `apps/desktop/release/`, `apps/desktop/sidecar/`.

## Trust boundaries

Zonas sensibles:

- preload: no ampliar superficie sin necesidad;
- HTTP LAN: opt-in, password, sin TLS propio;
- updater: GitHub HTTPS + SHA256, sin firma GPG por ahora;
- contexto: no filtrar valores de env ni secretos;
- agentes: usar CLIs externas sin acoplar LoLTerm a un proveedor.

## Regla de diseño

LoLTerm no reemplaza herramientas maduras: las abre, organiza y contextualiza dentro de terminales reales.