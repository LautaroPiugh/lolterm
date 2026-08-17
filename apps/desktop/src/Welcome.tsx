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
import { displayVersion, eraLabel } from "./version";
import type { Snapshot } from "./types";

const CLI_HINT: Record<string, string> = {
  nvim: "editor",
  lazygit: "git",
  btop: "monitor",
  yazi: "archivos",
  codex: "worktree + contexto",
  claude: "worktree + contexto",
  opencode: "worktree + contexto",
  gemini: "worktree + contexto",
  cline: "worktree + contexto",
};

function CliIcon({ name }: { name: string }) {
  if (name === "lazygit") return <GitBranch size={14} />;
  if (name === "yazi") return <FileCode size={14} />;
  if (["codex", "claude", "opencode", "gemini", "cline"].includes(name)) return <Sparkles size={14} />;
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
}) {
  const others = (snap.workspaces ?? []).filter((ws) => !ws.current);
  const tools = snap.run_clis ?? [];
  const machines = snap.machines ?? [];
  const presets = (snap.presets ?? []).slice(0, 4);
  const startup = (snap.startup ?? []).map((cmd) => cmd.program).join(" · ");
  const git = snap.git;

  return (
    <div className="welcome">
      <header className="welcome-hero">
        <img className="welcome-mark" src={`${import.meta.env.BASE_URL}icon.png`} alt="" width={56} height={56} />
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
          {snap.meta?.git_remote ? ` · ${snap.meta.git_remote}` : ""}
          {snap.meta?.stack?.length ? ` · ${snap.meta.stack.join(", ")}` : ""}
        </p>
        {git ? (
          <p className="welcome-git">
            <span className="diff-chip diff-add">+{git.staged + git.untracked}</span>
            <span className="diff-chip diff-del">−{git.unstaged}</span>
            {git.ahead > 0 ? <span className="welcome-git-extra">↑{git.ahead}</span> : null}
            {git.behind > 0 ? <span className="welcome-git-extra">↓{git.behind}</span> : null}
            <button type="button" className="welcome-inline" onClick={() => onRun("lazygit")} disabled={!tools.some((c) => c.name === "lazygit" && c.available)}>
              lazygit
            </button>
          </p>
        ) : null}
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
            Paleta de comandos
            <span className="welcome-chord">Ctrl-B</span>
          </button>
          <button type="button" className="welcome-action" onClick={onSsh}>
            <Server size={14} />
            Conectar SSH…
          </button>
          <button type="button" className="welcome-action" onClick={onTs}>
            <Cloud size={14} />
            Tailscale…
          </button>
        </section>
        <section>
          <h2>Herramientas</h2>
          {tools.length === 0 ? (
            <p className="welcome-empty">nada en el catálogo de CLIs</p>
          ) : (
            tools.map((cli) => (
              <button
                key={cli.name}
                type="button"
                className="welcome-action"
                disabled={!cli.available}
                title={cli.available ? `abrir ${cli.name}` : `${cli.name} no está en PATH`}
                onClick={() => onRun(cli.name)}
              >
                <CliIcon name={cli.name} />
                <span className="welcome-ws">{cli.name}</span>
                <span className="welcome-ws-path">{cli.available ? (CLI_HINT[cli.name] ?? "CLI") : "no en PATH"}</span>
              </button>
            ))
          )}
          {presets.length > 0 ? (
            <>
              <h2 className="welcome-subhead">Layouts</h2>
              {presets.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  className="welcome-action"
                  title={preset.hint}
                  onClick={() => onPreset(preset.id)}
                >
                  <Columns size={14} />
                  <span className="welcome-ws">{preset.name}</span>
                  <span className="welcome-ws-path">{preset.hint}</span>
                </button>
              ))}
            </>
          ) : null}
          {(snap.agent_log ?? []).length > 0 ? (
            <>
              <h2 className="welcome-subhead">Agentes recientes</h2>
              {(snap.agent_log ?? []).slice(0, 5).map((row) => (
                <button
                  key={`${row.ts}-${row.program}-${row.worktree ?? ""}`}
                  type="button"
                  className="welcome-action"
                  onClick={() => onRun(row.program)}
                >
                  <Sparkles size={14} />
                  <span className="welcome-ws">{row.program}</span>
                  <span className="welcome-ws-path">{row.worktree ?? row.workspace}</span>
                </button>
              ))}
            </>
          ) : null}
        </section>
        <section>
          <h2>Recientes</h2>
          {others.length === 0 ? (
            <p className="welcome-empty">no hay otros workspaces en el catálogo</p>
          ) : (
            others.slice(0, 8).map((ws) => (
              <button
                key={ws.root}
                type="button"
                className="welcome-action"
                onClick={() => onOpenWorkspace(ws.root)}
              >
                <span className="welcome-ws">{ws.name}</span>
                <span className="welcome-ws-path">{ws.root_label ?? ws.root}</span>
              </button>
            ))
          )}
          {machines.length > 0 ? (
            <>
              <h2 className="welcome-subhead">Máquinas</h2>
              {machines.slice(0, 8).map((machine) => (
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
        <ul className="welcome-keys">
          <li>
            <kbd>Ctrl-Tab</kbd> cicla tabs
          </li>
          <li>
            <kbd>Ctrl-Alt-[ ]</kbd> workspaces
          </li>
          <li>
            <kbd>Ctrl-Alt-V / S</kbd> split
          </li>
          <li>Arrastrá una pestaña al borde para partir el layout</li>
        </ul>
        {startup ? <p className="welcome-startup">al abrir: {startup}</p> : null}
        {snap.meta?.notes ? <p className="welcome-notes">{snap.meta.notes}</p> : null}
        <p className="welcome-era">{eraLabel(snap.version)}</p>
      </footer>
    </div>
  );
}
