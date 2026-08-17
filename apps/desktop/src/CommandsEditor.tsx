import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Command, Plus, Terminal, X } from "./icons";
import { eventChord } from "./chords";
import type { CommandHit } from "./types";

export type ExtCommand = {
  id: string;
  slash: string;
  hint: string;
  run: string;
  args?: string[];
};

type Binding = { chord: string; command: string };

type Props = {
  commands: ExtCommand[];
  bindings: Binding[];
  catalog: CommandHit[];
  commandsPath: string;
  keybindingsPath: string;
  onSave: (draft: { slash: string; hint: string; run: string; args: string[]; id?: string }) => void;
  onRemove: (id: string) => void;
  onBind: (chord: string, command: string) => void;
  onResetKeys: () => void;
  onOpenFile: (file: "commands" | "keybindings") => void;
  onClose: () => void;
};

type Tab = "keys" | "commands";

type Capture = { command: string };
type Conflict = {
  chord: string;
  command: string;
  takenBy: string;
};

function labelOf(id: string, catalog: CommandHit[], commands: ExtCommand[]) {
  const hit = catalog.find((item) => item.id === id || item.slash === id);
  if (hit) return `/${hit.slash} · ${hit.hint}`;
  const ext = commands.find((item) => item.id === id);
  if (ext) return `/${ext.slash} · ${ext.hint || ext.run}`;
  return id;
}

export function CommandsEditor({
  commands,
  bindings,
  catalog,
  commandsPath,
  keybindingsPath,
  onSave,
  onRemove,
  onBind,
  onResetKeys,
  onOpenFile,
  onClose,
}: Props) {
  const [tab, setTab] = useState<Tab>("keys");
  const [slash, setSlash] = useState("");
  const [hint, setHint] = useState("");
  const [run, setRun] = useState("");
  const [args, setArgs] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [capture, setCapture] = useState<Capture | null>(null);
  const [conflict, setConflict] = useState<Conflict | null>(null);

  const labels = useMemo(() => {
    return (id: string) => labelOf(id, catalog, commands);
  }, [catalog, commands]);

  const sortedBindings = useMemo(
    () => [...bindings].sort((a, b) => a.chord.localeCompare(b.chord)),
    [bindings],
  );

  function startCapture(command: string) {
    setConflict(null);
    setCapture({ command });
  }

  function commit(chord: string, command: string) {
    setConflict(null);
    setCapture(null);
    onBind(chord, command);
  }

  function considerChord(chord: string, command: string) {
    const current = bindings.find((item) => item.command === command);
    if (current?.chord === chord) {
      setCapture(null);
      return;
    }
    const taken = bindings.find((item) => item.chord === chord);
    if (taken && taken.command !== command) {
      setCapture(null);
      setConflict({ chord, command, takenBy: taken.command });
      return;
    }
    commit(chord, command);
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        if (conflict) {
          setConflict(null);
          return;
        }
        if (capture) {
          setCapture(null);
          return;
        }
        onClose();
        return;
      }
      if (!capture || conflict) return;
      if (event.key === "Control" || event.key === "Alt" || event.key === "Shift" || event.key === "Meta") {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      considerChord(eventChord(event), capture.command);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [bindings, capture, conflict, onBind, onClose]);

  function fill(cmd: ExtCommand) {
    setEditing(cmd.id);
    setSlash(cmd.slash);
    setHint(cmd.hint);
    setRun(cmd.run);
    setArgs((cmd.args ?? []).join(" "));
  }

  function clearForm() {
    setEditing(null);
    setSlash("");
    setHint("");
    setRun("");
    setArgs("");
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    onSave({
      id: editing ?? undefined,
      slash,
      hint,
      run,
      args: args
        .split(/\s+/)
        .map((part) => part.trim())
        .filter(Boolean),
    });
    clearForm();
  }

  const overlay = conflict ? (
    <div className="commands-overlay" role="alertdialog" aria-modal="true">
      <div className="commands-dialog">
        <p className="commands-dialog-kicker">atajo en uso</p>
        <p className="commands-dialog-chord">{conflict.chord}</p>
        <p>
          Ya está asignado a <strong>{labels(conflict.takenBy)}</strong>.
        </p>
        <p>
          ¿Reemplazarlo por <strong>{labels(conflict.command)}</strong>?
        </p>
        <div className="commands-dialog-actions">
          <button type="button" onClick={() => setConflict(null)}>
            Cancelar
          </button>
          <button type="button" className="on" onClick={() => commit(conflict.chord, conflict.command)}>
            Reemplazar
          </button>
        </div>
      </div>
    </div>
  ) : capture ? (
    <div className="commands-overlay" role="dialog" aria-modal="true">
      <div className="commands-dialog">
        <p className="commands-dialog-kicker">nueva combinación</p>
        <p className="commands-dialog-chord pulse">apretá las teclas</p>
        <p>
          Atajo para <strong>{labels(capture.command)}</strong>
        </p>
        <p className="hint">Esc cancela</p>
      </div>
    </div>
  ) : null;

  return (
    <div className="commands-editor" onClick={(e) => e.stopPropagation()}>
      {overlay}
      <header className="commands-head">
        <Command size={14} color="var(--brand)" />
        <h2>Comandos y atajos</h2>
        <div className="commands-tabs">
          <button type="button" className={tab === "keys" ? "on" : ""} onClick={() => setTab("keys")}>
            Atajos
          </button>
          <button type="button" className={tab === "commands" ? "on" : ""} onClick={() => setTab("commands")}>
            Custom
          </button>
        </div>
      </header>
      <p className="commands-note">Doble clic en una fila para cambiar el atajo. Si la combinación ya existe, LoLTerm avisa antes de pisarla.</p>

      {tab === "keys" && (
        <>
          <p className="commands-path" title={keybindingsPath}>
            {keybindingsPath}
            <button type="button" className="linkish" onClick={() => onOpenFile("keybindings")}>
              abrir archivo
            </button>
            <button type="button" className="linkish" onClick={onResetKeys}>
              defaults
            </button>
          </p>
          <div className="commands-list">
            {sortedBindings.map((item) => (
              <div
                key={item.chord}
                className="commands-row"
                title="Doble clic para reasignar"
                onDoubleClick={() => startCapture(item.command)}
              >
                <span className="chord-label">{item.chord}</span>
                <span className="commands-main">
                  <span>{labels(item.command)}</span>
                  <span className="hint">{item.command}</span>
                </span>
                <button type="button" className="icon-ghost" title="quitar" onClick={() => onBind(item.chord, "")}>
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        </>
      )}

      {tab === "commands" && (
        <>
          <p className="commands-path" title={commandsPath}>
            {commandsPath}
            <button type="button" className="linkish" onClick={() => onOpenFile("commands")}>
              abrir archivo
            </button>
          </p>
          <form className="commands-form" onSubmit={submit}>
            <input value={slash} onChange={(e) => setSlash(e.target.value)} placeholder="slash (htop)" spellCheck={false} />
            <input value={run} onChange={(e) => setRun(e.target.value)} placeholder="binario (htop)" spellCheck={false} />
            <input value={hint} onChange={(e) => setHint(e.target.value)} placeholder="hint" spellCheck={false} />
            <input value={args} onChange={(e) => setArgs(e.target.value)} placeholder="args (sin flags)" spellCheck={false} />
            <button type="submit" className="open-folder-btn" disabled={!run.trim()}>
              <Plus size={12} />
              {editing ? "Actualizar" : "Agregar"}
            </button>
            {editing && (
              <button type="button" className="linkish" onClick={clearForm}>
                cancelar
              </button>
            )}
          </form>
          <div className="commands-list">
            {commands.length === 0 && <p className="commands-empty">Todavía no hay `ext.*`. Agregá uno arriba.</p>}
            {commands.map((cmd) => {
              const chord = bindings.find((item) => item.command === cmd.id)?.chord;
              return (
                <div
                  key={cmd.id}
                  className="commands-row"
                  title="Clic para editar · doble clic para el atajo"
                  onDoubleClick={() => startCapture(cmd.id)}
                >
                  <Terminal size={12} color="var(--muted)" />
                  <button type="button" className="commands-main" onClick={() => fill(cmd)}>
                    <span>
                      /{cmd.slash} · {cmd.hint || cmd.run}
                    </span>
                    <span className="hint">{cmd.id}</span>
                  </button>
                  <span className="chord-label">{chord ?? "sin atajo"}</span>
                  <button type="button" className="icon-ghost" title="quitar" onClick={() => onRemove(cmd.id)}>
                    <X size={12} />
                  </button>
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}
