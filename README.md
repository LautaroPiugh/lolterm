# LoLTerm

LoLTerm es un **workspace de terminales**. La unidad de trabajo no es el archivo: es el proceso que corre en una terminal real (bash, nvim, lazygit, ssh, Codex, lo que uses).

> LoLTerm es el entorno desde el que trabajo, no la herramienta con la que hago cada trabajo.

No nació para ser “otra terminal bonita”. La idea es instalarlo en una máquina y usarlo como sitio desde el que abrís proyectos, CLIs, editores, Git, agentes de IA y otras computadoras.

Hoy el paquete oficial es un **`.deb` para Ubuntu**.

## Qué hace (y qué no)

LoLTerm **organiza** herramientas que ya existen. No las reemplaza.

| LoLTerm no es… | Lo que hace en su lugar |
| --- | --- |
| un IDE ni un editor propio | overlay para leer/guardar un archivo; el default sigue siendo nvim / `$EDITOR` en un PTY |
| un cliente Git completo | vista tipo SCM (stage, commit, fetch, pull --ff-only); lazygit para el resto. Sin merge UI ni force-push |
| un chat o runtime de IA | abre Codex, Claude Code, OpenCode, Gemini, Cline, Copilot en una terminal (worktree + contexto) |
| un fork de tmux | usa tmux en remoto para no perder la sesión |
| un cliente SSH distinto | usa el `ssh` del sistema (y Tailscale si lo tenés) |
| una tienda de software | Ajustes instala CLIs conocidas corriendo el comando oficial en un PTY |

Adentro, cada panel es un **PTY**: el programa cree que está en una terminal de verdad (colores, nvim, fzf, resize). Varios paneles y pestañas conviven en la misma ventana.

```text
                LoLTerm
                   │
       ┌───────────┼────────────┐
       │           │            │
    proyectos   máquinas     contexto
       │           │            │
       ↓           ↓            ↓
    terminales   SSH/tmux    agentes CLI
       │
  nvim / lazygit / shell / …
```

Todo corre **en tu máquina**. No hay cuenta, ni nube de LoLTerm, ni backend obligatorio. La config portable vive en `~/.config/lolterm/`.

## Cómo está armado

Hay tres piezas que el usuario ve como “LoLTerm”:

1. **La ventana** — pestañas, splits, paleta, explorer, Git, Ajustes, temas.
2. **El core** — crea las terminales, guarda el layout, habla con SSH y con la CLI.
3. **El comando `lolterm`** — el mismo sistema, desde otra terminal: abrir un proyecto, listar workspaces, preguntar el contexto.

Un **workspace** es un entorno recuperable: carpeta del proyecto, pestañas, paneles, a veces una máquina remota. Podés tener varios y ciclarlos. Si cerrás todas las pestañas, volvés a Inicio (proyectos, CLIs, máquinas).

**Local:** panel → terminal en esta PC.  
**Remoto:** panel → `ssh` → (opcional) tmux en la otra máquina, para que nvim no se muera si se corta la red.

La CLI `lolterm context` (y el archivo que ven los PTYs en `LOLTERM_CONTEXT`) expone carpeta, rama, procesos y, si nvim tiene un archivo abierto, `focused_file`. LoLTerm no llama a Anthropic ni OpenAI: **Quota** lee las CLIs instaladas (Codex `app-server`, `claude --print /usage`, Antigravity vía su API local mientras corre, Copilot vía `gh`, OpenCode Go, ClinePass). Hermes usa la configuración de provider/modelo que tenga el usuario y se abre como una CLI normal. El chip de media usa **playerctl** (MPRIS), no un clon de Spotify.

La barra de estado muestra rama, puertos, procesos y atención de agentes. En ventanas estrechas hay una **barra táctil** (Esc, Ctrl-C, flechas) para la terminal.

Podés sumar comandos, atajos, temas y ganchos con archivos TOML locales. No hay plugins de JavaScript.

## Instalar (Ubuntu)

1. Bajá el `.deb` de la [última release](https://github.com/LautaroPiugh/lolterm/releases/latest).
2. Instalálo:

```bash
sudo apt install ./LoLTerm-*-linux-amd64.deb
```

Queda en el menú de aplicaciones (icono del prompt `>`) y `lolterm` en el PATH.

Para actualizar: paleta `/update`, o el aviso cuando hay versión nueva. Se descarga el `.deb` de **esa** release, se comprueba el SHA256 y recién ahí se instala. No hace falta un token de GitHub.

## Uso

| Qué | Cómo |
| --- | --- |
| Paleta | `Ctrl-b` o `Ctrl-p` |
| Cambiar de tab | `Ctrl-Tab` / `Ctrl-Shift-Tab` |
| Partir el panel | `Ctrl-Alt-v` (derecha), `Ctrl-Alt-s` (abajo) |
| Workspaces | clic en el nombre, o `Ctrl-Alt-[` `]` |
| Reiniciar el panel | `Ctrl-Alt-r` |
| Ajustes (temas, CLIs, HTTP) | engranaje, `/settings` o `/theme` |
| Comandos y atajos | `Ctrl-Alt-,` o `/commands` |
| Git (SCM) | rail Git: staged / changes, commit (Ctrl+Enter), fetch, pull `--ff-only` |
| Archivo (explorer) | overlay para leer/guardar, o **nvim** (`$EDITOR`). `Ctrl+S` guarda. Autosave: `[editor] autowrite = true` en `config.toml` |
| REST | `+` → REST, o paleta `/rest`: archivos `.http` / `.rest` del repo; secretos desde `.env` local |
| Copiar al seleccionar | `/copy-select` (también Ctrl+Shift+C) |

El `+` pregunta qué abrir (shell, SSH, nvim, lazygit, btop, yazi, agentes). `gh` y `rg` se instalan desde Ajustes; no salen en el `+` porque no son una tab vacía. Arrastrá una pestaña al borde para partir el layout.

Desde Ajustes → **Herramientas** podés instalar o abrir CLIs conocidas (el comando corre en un PTY: apt, npm, cargo, go). Si ya está en PATH, **Abrir** lanza un panel. Paleta: `/nvim`, `/lazygit`, `/codex`, `/claude`, …

Desde otra terminal, el mismo workspace:

```bash
lolterm .
lolterm workspace list
lolterm ssh home
lolterm run nvim
lolterm context
lolterm panes
lolterm processes
```

## Máquinas remotas

La contraseña la pide `ssh` en el panel. Ejemplo en `~/.config/lolterm/config.toml`:

```toml
[remote]
user = "dev"
tmux = "lolterm"

[[machines]]
name = "home"
target = "home.example.ts.net"
user = "dev"
kind = "tailscale"
```

Si el enlace se corta, LoLTerm intenta reconectar **una vez** (tmux `-A` recupera la sesión remota).

HTTP LAN es **opt-in** (Ajustes → Red): vista web del mismo core en LAN/VPN, password en `data_dir`, sin TLS propio. Remoto de verdad sigue siendo SSH + tmux.

## Configuración

| Dónde | Qué |
| --- | --- |
| `~/.config/lolterm/` | temas, atajos, comandos, workspaces, extensiones TOML |
| `$XDG_RUNTIME_DIR/lolterm/` | estado vivo (socket de contexto); no hace falta sincronizarlo |

SSH usa las claves y el agent del sistema. No guardes secretos en archivos que copies entre PCs.

## Licencia

[MIT](LICENSE). Podés usar, copiar, modificar y redistribuir el software; el archivo `LICENSE` es el texto legal.
