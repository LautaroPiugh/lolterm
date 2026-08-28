# Privacidad

LoLTerm está diseñado como aplicación local-first.

## Resumen

- No requiere cuenta de LoLTerm.
- No tiene telemetría automática.
- No envía el contenido de tus terminales a un servidor de LoLTerm.
- No implementa un modelo de IA propio ni manda prompts a proveedores por sí mismo.
- Las CLIs que abras dentro de un PTY conservan su propio comportamiento de red.

## Datos locales

Ubicaciones principales:

| Ruta | Uso |
| --- | --- |
| `~/.config/lolterm/` | configuración portable: temas, atajos, comandos, workspaces, extensiones TOML |
| `$XDG_RUNTIME_DIR/lolterm/` | estado vivo: socket del mux y contexto temporal |
| `$XDG_DATA_HOME/lolterm/` | datos locales: worktrees de agentes, historial local, password HTTP |
| userData de Electron | estado de ventana y datos del renderer |

No sincronices archivos con secretos. Si usás Git/dotfiles para `~/.config/lolterm/`, revisá los TOML antes de publicarlos.

## Contexto del workspace

`lolterm context` y `LOLTERM_CONTEXT` exponen contexto útil para herramientas CLI:

- workspace actual;
- carpeta raíz;
- rama Git y remoto sanitizado;
- procesos/panes;
- nombres de variables de entorno permitidas;
- archivo enfocado cuando se puede detectar.

No deben incluir valores completos de variables de entorno ni claves que parezcan secretos (`TOKEN`, `PASSWORD`, `SECRET`, `API_KEY`, etc.).

## REST client y `.env`

El REST client puede expandir variables desde `.env` local para archivos `.http` / `.rest`. Esos `.env` están ignorados por Git. Publicá sólo `.env.example` con nombres de variables y valores ficticios.

## Updates

El updater consulta GitHub Releases del repositorio configurado. Esa consulta llega a GitHub e incluye datos normales de una request HTTPS, como IP y User-Agent. No necesita token para repos públicos.

## Diagnósticos e issues

El panel de diagnóstico guarda avisos localmente. Al abrir un issue, LoLTerm prepara un texto para ayudarte a reportar el problema. Revisalo antes de enviarlo: puede incluir versión, tema, tipo de navegador/Electron y mensajes de error recientes.

## Agentes externos

Si abrís Codex, Claude Code, OpenCode, Gemini, Cline, Copilot u otra CLI, esa herramienta puede comunicarse con su proveedor según su propia configuración. LoLTerm sólo la aloja dentro de un PTY y puede pasar contexto local mediante variables/archivos.