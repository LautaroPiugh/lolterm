# LoLTerm

LoLTerm es un **workspace de terminales**. La unidad de trabajo no es el archivo: es el proceso que corre en una terminal real (bash, nvim, lazygit, ssh, Codex, lo que uses).

> LoLTerm es el entorno desde el que trabajo, no la herramienta con la que hago cada trabajo.

No nació para ser “otra terminal bonita”. La idea es instalarlo en una máquina y usarlo como sitio desde el que abrís proyectos, CLIs, editores, Git, agentes de IA y otras computadoras.

Hoy el paquete oficial es un **`.deb` para Ubuntu**.

## Qué hace (y qué no)

LoLTerm **organiza** herramientas que ya existen. No las reemplaza.

| LoLTerm no es… | Lo que hace en su lugar |
| --- | --- |
| un IDE ni un editor propio | abre nvim, Helix o `$EDITOR` en un panel |
| un cliente Git | conoce la rama y puede abrir lazygit |
| un chat o runtime de IA | abre Codex, Claude Code, OpenCode, … en una terminal |
| un fork de tmux | usa tmux en remoto para no perder la sesión |
| un cliente SSH distinto | usa el `ssh` del sistema (y Tailscale si lo tenés) |

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

1. **La ventana** — pestañas, splits, paleta, explorer, temas.
2. **El core** — crea las terminales, guarda el layout, habla con SSH y con la CLI.
3. **El comando `lolterm`** — el mismo sistema, desde otra terminal: abrir un proyecto, listar workspaces, preguntar el contexto.

Un **workspace** es un entorno recuperable: carpeta del proyecto, pestañas, paneles, a veces una máquina remota. Podés tener varios y ciclarlos. Si cerrás todas las pestañas, volvés a Inicio (proyectos, CLIs, máquinas).

**Local:** panel → terminal en esta PC.  
**Remoto:** panel → `ssh` → (opcional) tmux en la otra máquina, para que nvim no se muera si se corta la red.

La CLI `lolterm context` (y variables como `LOLTERM_ROOT`) exponen carpeta, rama y procesos abiertos, para que un agente CLI sepa dónde está sin un protocolo propio de LoLTerm.

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
| Comandos y atajos | `Ctrl-Alt-,` o `/commands` |

El `+` pregunta qué abrir (shell, SSH, agentes). Arrastrá una pestaña al borde para partir el layout.

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

## Configuración

| Dónde | Qué |
| --- | --- |
| `~/.config/lolterm/` | temas, atajos, comandos, workspaces, extensiones TOML |
| `$XDG_RUNTIME_DIR/lolterm/` | estado vivo (socket de contexto); no hace falta sincronizarlo |

SSH usa las claves y el agent del sistema. No guardes secretos en archivos que copies entre PCs.

## Licencia

[MIT](LICENSE). Podés usar, copiar, modificar y redistribuir el software; el archivo `LICENSE` es el texto legal.
