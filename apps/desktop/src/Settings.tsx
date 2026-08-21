import { useEffect, useState } from "react";
import {
  Check,
  Columns,
  Command,
  Plus,
  Settings as Gear,
  Sparkles,
  Terminal,
  X,
} from "./icons";
import { THEMES, swatchGradient } from "./themes";
import type { Snapshot } from "./types";

export type SettingsTab = "look" | "tools" | "workspace" | "net";

type Call = (method: string, params?: unknown) => Promise<unknown>;

export function Settings({
  snap,
  tab,
  onTab,
  call,
  onClose,
  onOpenCommands,
  onUpdate,
}: {
  snap: Snapshot;
  tab: SettingsTab;
  onTab: (tab: SettingsTab) => void;
  call: Call;
  onClose: () => void;
  onOpenCommands: () => void;
  onUpdate: () => void;
}) {
  const tools = snap.tools ?? snap.agent_tools ?? [];
  const clis = tools.filter((tool) => tool.kind !== "agent");
  const agents = tools.filter((tool) => tool.kind === "agent" || tool.kind == null);
  const [envKey, setEnvKey] = useState("");
  const [envVal, setEnvVal] = useState("");
  const [notes, setNotes] = useState(snap.meta?.notes ?? "");
  const [httpPass, setHttpPass] = useState("");

  useEffect(() => {
    setNotes(snap.meta?.notes ?? "");
  }, [snap.meta?.notes]);

  function install(name: string) {
    void call("installAgent", { name });
    onClose();
  }

  return (
    <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
      <header className="settings-head">
        <Gear size={14} color="var(--brand)" />
        <div>
          <h2>Ajustes</h2>
          <p className="settings-kicker">workspace · no es un marketplace</p>
        </div>
        <button type="button" className="settings-close" title="Cerrar" onClick={onClose}>
          <X size={14} />
        </button>
      </header>
      <nav className="settings-tabs">
        {(
          [
            ["look", "Apariencia"],
            ["tools", "Herramientas"],
            ["workspace", "Workspace"],
            ["net", "Red"],
          ] as const
        ).map(([id, label]) => (
          <button key={id} type="button" className={tab === id ? "on" : ""} onClick={() => onTab(id)}>
            {label}
          </button>
        ))}
      </nav>
      <div className="settings-body">
        {tab === "look" && (
          <>
            <p className="settings-lead">Tema de chrome y xterm. Los packs extra viven en ~/.config/lolterm/themes.</p>
            <ThemePicker
              current={snap.theme}
              themes={snap.themes ?? THEMES}
              onPick={(id) => void call("setTheme", { theme: id })}
            />
            <div className="settings-row-actions">
              <button type="button" className="settings-ghost" onClick={onOpenCommands}>
                <Command size={12} />
                Comandos y atajos
                <span>Ctrl-Alt-,</span>
              </button>
              <button type="button" className="settings-ghost" onClick={onUpdate}>
                <Sparkles size={12} />
                Buscar actualización
                <span>/update</span>
              </button>
            </div>
          </>
        )}
        {tab === "tools" && (
          <>
            <p className="settings-lead">
              Instalar abre un PTY y corre el comando de esa herramienta (apt, npm, cargo, go). No hay tienda propia.
            </p>
            <ToolGroup
              title="CLIs"
              tools={clis}
              onInstall={install}
              onOpen={(name) => {
                void call("run", { program: name, args: [] });
                onClose();
              }}
            />
            <ToolGroup
              title="Agentes"
              tools={agents}
              onInstall={install}
              onOpen={(name) => {
                void call("run", { program: name, args: [] });
                onClose();
              }}
            />
          </>
        )}
        {tab === "workspace" && (
          <>
            <h3 className="settings-h">Layouts</h3>
            <div className="settings-preset-grid">
              {(snap.presets ?? []).map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  className="settings-preset"
                  onClick={() => void call("applyPreset", { id: preset.id })}
                >
                  <Columns size={13} color="var(--brand)" />
                  <strong>{preset.name}</strong>
                  <span>{preset.hint}</span>
                </button>
              ))}
            </div>
            <h3 className="settings-h">Al abrir</h3>
            <p className="settings-lead">CLIs que LoLTerm lanza al recuperar este workspace.</p>
            <div className="settings-chips">
              {(snap.startup ?? []).map((cmd) => (
                <button
                  key={cmd.program}
                  type="button"
                  className="settings-chip on"
                  title="quitar"
                  onClick={() => void call("removeStartup", { program: cmd.program })}
                >
                  {cmd.program}
                  <X size={10} />
                </button>
              ))}
              {snap.run_clis
                .filter(
                  (cli) => cli.available && !(snap.startup ?? []).some((cmd) => cmd.program === cli.name),
                )
                .map((cli) => (
                  <button
                    key={cli.name}
                    type="button"
                    className="settings-chip"
                    onClick={() => void call("addStartup", { program: cli.name, args: [] })}
                  >
                    <Plus size={10} />
                    {cli.name}
                  </button>
                ))}
            </div>
            <h3 className="settings-h">Entorno</h3>
            {(snap.env ?? []).map((item) => (
              <button
                key={item.key}
                type="button"
                className="settings-env-hit"
                title="quitar"
                onClick={() => void call("removeEnv", { key: item.key })}
              >
                <code>{item.key}</code>
                <span>quitar</span>
              </button>
            ))}
            <form
              className="env-form"
              onSubmit={(event) => {
                event.preventDefault();
                const key = envKey.trim();
                if (!key) return;
                void call("setEnv", { key, value: envVal }).then(() => {
                  setEnvKey("");
                  setEnvVal("");
                });
              }}
            >
              <input value={envKey} onChange={(e) => setEnvKey(e.target.value)} placeholder="NOMBRE" spellCheck={false} autoComplete="off" />
              <input value={envVal} onChange={(e) => setEnvVal(e.target.value)} placeholder="valor" spellCheck={false} autoComplete="off" />
              <button type="submit" className="open-folder-btn" disabled={!envKey.trim()}>
                Guardar
              </button>
            </form>
            <h3 className="settings-h">Proyecto</h3>
            <div className="meta-chips">
              {(snap.meta?.stack ?? []).map((item) => (
                <span key={item} className="meta-chip">
                  {item}
                </span>
              ))}
              {snap.meta?.git_remote && <span className="meta-chip">{snap.meta.git_remote}</span>}
            </div>
            <form
              className="env-form"
              onSubmit={(event) => {
                event.preventDefault();
                void call("setNotes", { notes });
              }}
            >
              <textarea
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="nota (sin secretos)"
                rows={2}
                spellCheck={false}
              />
              <button type="submit" className="open-folder-btn" disabled={notes.trim() === (snap.meta?.notes ?? "")}>
                Guardar nota
              </button>
            </form>
          </>
        )}
        {tab === "net" && (
          <>
            <p className="settings-lead">
              HTTP opt-in para una vista web del mismo core. LAN/VPN, password en data_dir, sin TLS propio. Remoto de
              verdad sigue siendo SSH + tmux.
            </p>
            <p className="hint">puerto 47832 · {snap.http?.enabled ? `activo · ${snap.http.bind}` : "apagado"}</p>
            <form
              className="env-form"
              onSubmit={(event) => {
                event.preventDefault();
                void call("setHttp", { enabled: true, password: httpPass });
              }}
            >
              <input
                type="password"
                value={httpPass}
                placeholder="password ≥ 8"
                onChange={(e) => setHttpPass(e.target.value)}
              />
              <button type="submit" className="open-folder-btn">
                {snap.http?.enabled ? "actualizar" : "activar"}
              </button>
            </form>
            {snap.http?.enabled ? (
              <button type="button" className="settings-ghost" onClick={() => void call("setHttp", { enabled: false })}>
                desactivar HTTP
              </button>
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}

function ToolGroup({
  title,
  tools,
  onInstall,
  onOpen,
}: {
  title: string;
  tools: NonNullable<Snapshot["tools"]>;
  onInstall: (name: string) => void;
  onOpen: (name: string) => void;
}) {
  if (tools.length === 0) return null;
  return (
    <section className="settings-tools">
      <h3 className="settings-h">{title}</h3>
      {tools.map((tool) => (
        <div key={tool.name} className="settings-tool">
          <div className="settings-tool-icon">{tool.kind === "agent" ? <Sparkles size={13} /> : <Terminal size={13} />}</div>
          <div className="settings-tool-copy">
            <strong>{tool.name}</strong>
            <span>{tool.hint ?? tool.install}</span>
          </div>
          {tool.available ? (
            <>
              <button type="button" className="settings-install" title={`abrir ${tool.name}`} onClick={() => onOpen(tool.name)}>
                Abrir
              </button>
              <button type="button" className="settings-ghost tiny" title={tool.install} onClick={() => onInstall(tool.name)}>
                actualizar
              </button>
            </>
          ) : (
            <button type="button" className="settings-install" title={tool.install} onClick={() => onInstall(tool.name)}>
              Instalar
            </button>
          )}
        </div>
      ))}
    </section>
  );
}

export function ThemePicker({
  current,
  themes,
  onPick,
}: {
  current: string;
  themes: { id: string; label: string; hint: string }[];
  onPick: (id: string) => void;
}) {
  return (
    <div className="theme-grid">
      {themes.map((item) => (
        <button
          key={item.id}
          type="button"
          className={current === item.id ? "theme-tile on" : "theme-tile"}
          onClick={() => onPick(item.id)}
        >
          <span className="theme-tile-swatch" style={{ background: swatchGradient(item.id) }} />
          <span className="theme-tile-copy">
            <strong>{item.label}</strong>
            <span>{item.hint}</span>
          </span>
          {current === item.id ? <Check size={13} color="var(--brand)" /> : null}
        </button>
      ))}
    </div>
  );
}
