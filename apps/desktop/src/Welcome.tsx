import {
  Cloud,
  Columns,
  Command,
  FileCode,
  FolderPlus,
  GitBranch,
  Network,
  Plus,
  Server,
  Sparkles,
  Terminal,
} from "./icons";
import { displayVersion } from "./version";
import type { Snapshot } from "./types";

const CLI_HINT: Record<string, string> = {
  nvim: "editor",
  lazygit: "git",
  btop: "monitor",
  yazi: "archivos",
  fzf: "fuzzy",
  gh: "GitHub",
  tmux: "mux",
  rg: "ripgrep",
  delta: "diffs",
  codex: "agente",
  claude: "agente",
  opencode: "agente",
  gemini: "agente",
  cline: "agente",
  copilot: "agente",
};

const DAILY = ["nvim", "lazygit", "btop", "yazi", "fzf"];
const AGENTS = ["codex", "claude", "opencode", "gemini", "cline", "copilot"];

function CliIcon({ name }: { name: string }) {
  if (name === "lazygit") return <GitBranch size={14} />;
  if (name === "yazi") return <FileCode size={14} />;
  if (AGENTS.includes(name)) return <Sparkles size={14} />;
  return <Terminal size={14} />;
}

export function Welcome({
  snap,
  onNewTab,
  onOpenFolder,
  onPalette,
  onOpenWorkspace,
  onRun,
  onFiles,
  onSsh,
  onTs,
  onConnectMachine,
  onPreset,
  onTools,
}: {
  snap: Snapshot;
  onNewTab: () => void;
  onOpenFolder: () => void;
  onPalette: () => void;
  onOpenWorkspace: (path: string) => void;
  onRun: (program: string) => void;
  onFiles: () => void;
  onSsh: () => void;
  onTs: () => void;
  onConnectMachine: (target: string) => void;
  onPreset: (id: string) => void;
  onTools: () => void;
}) {
  const others = (snap.workspaces ?? []).filter((ws) => !ws.current);
  const tools = snap.run_clis ?? [];
  const ready = tools.filter((cli) => cli.available);
  const daily = DAILY.map((name) => ready.find((cli) => cli.name === name)).filter(
    (cli): cli is NonNullable<typeof cli> => Boolean(cli),
  );
  const agents = ready.filter((cli) => AGENTS.includes(cli.name));
  const nvimMissing = tools.some((cli) => cli.name === "nvim" && !cli.available);
  const machines = snap.machines ?? [];
  const presets = (snap.presets ?? []).slice(0, 5);
  const git = snap.git;

  return (
    <div className="welcome">
      <header className="welcome-hero">
        <p className="welcome-kicker">LoLTerm {displayVersion(snap.version)}</p>
        <h1>{snap.name}</h1>
        <p className="welcome-sub">
          {snap.branch ? (
            <>
              <GitBranch size={12} color="var(--brand)" /> {snap.branch}
            </>
          ) : (
            "sin git"
          )}
          {git ? (
            <>
              <span className="diff-chip diff-add">+{git.staged + git.untracked}</span>
              <span className="diff-chip diff-del">−{git.unstaged}</span>
            </>
          ) : null}
        </p>
        <p className="welcome-path">{snap.root}</p>
      </header>
      <div className="welcome-grid">
        <section>
          <h2>Inicio</h2>
          <button type="button" className="welcome-action" onClick={onNewTab}>
            <Plus size={14} />
            Nueva terminal
            <span className="welcome-chord">Ctrl-Alt-N</span>
          </button>
          <button type="button" className="welcome-action" onClick={onOpenFolder}>
            <FolderPlus size={14} />
            Abrir carpeta…
          </button>
          <button type="button" className="welcome-action" onClick={onFiles}>
            <FileCode size={14} />
            Buscar archivo
          </button>
          <button type="button" className="welcome-action" onClick={onPalette}>
            <Command size={14} />
            Paleta
            <span className="welcome-chord">Ctrl-B</span>
          </button>
          <div className="welcome-inline-row">
            <button type="button" className="welcome-mini" onClick={onSsh}>
              <Server size={12} />
              SSH
            </button>
            <button type="button" className="welcome-mini" onClick={onTs}>
              <Cloud size={12} />
              Tailscale
            </button>
          </div>
          {presets.length > 0 ? (
            <>
              <h2 className="welcome-subhead">Layouts</h2>
              <div className="welcome-preset-row">
                {presets.map((preset) => (
                  <button
                    key={preset.id}
                    type="button"
                    className="welcome-preset"
                    title={preset.hint}
                    onClick={() => onPreset(preset.id)}
                  >
                    <Columns size={12} />
                    {preset.name}
                  </button>
                ))}
              </div>
            </>
          ) : null}
        </section>
        <section>
          <h2>En PATH</h2>
          {daily.length === 0 && agents.length === 0 ? (
            <p className="welcome-empty">ninguna CLI del catálogo</p>
          ) : (
            <div className="welcome-tool-row">
              {[...daily, ...agents].map((cli) => (
                <button
                  key={cli.name}
                  type="button"
                  className="welcome-tool"
                  title={CLI_HINT[cli.name] ?? "CLI"}
                  onClick={() => onRun(cli.name)}
                >
                  <CliIcon name={cli.name} />
                  {cli.name}
                </button>
              ))}
            </div>
          )}
          {nvimMissing ? (
            <button type="button" className="welcome-miss" onClick={onTools}>
              nvim no está en PATH · instalar en Ajustes
            </button>
          ) : null}
          {others.length > 0 ? (
            <>
              <h2 className="welcome-subhead">Recientes</h2>
              {others.slice(0, 5).map((ws) => (
                <button
                  key={ws.root}
                  type="button"
                  className="welcome-action"
                  onClick={() => onOpenWorkspace(ws.root)}
                >
                  <span className="welcome-ws">{ws.name}</span>
                  <span className="welcome-ws-path">{ws.root_label ?? ws.root}</span>
                </button>
              ))}
            </>
          ) : null}
          {machines.length > 0 ? (
            <>
              <h2 className="welcome-subhead">Máquinas</h2>
              {machines.slice(0, 5).map((machine) => (
                <button
                  key={`${machine.kind}:${machine.target}`}
                  type="button"
                  className="welcome-action"
                  onClick={() => onConnectMachine(machine.target)}
                >
                  {machine.kind === "tailscale" ? <Network size={14} /> : <Server size={14} />}
                  <span className="welcome-ws">{machine.name}</span>
                  <span className="welcome-ws-path">{machine.target}</span>
                </button>
              ))}
            </>
          ) : null}
        </section>
      </div>
      <footer className="welcome-foot">
        <p className="welcome-keys-line">
          <kbd>Ctrl-Tab</kbd> tabs · <kbd>Ctrl-Alt-[ ]</kbd> workspaces · <kbd>Ctrl-Alt-V/S</kbd> split
        </p>
      </footer>
    </div>
  );
}
