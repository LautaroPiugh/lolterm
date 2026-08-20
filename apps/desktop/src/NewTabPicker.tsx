import { Cloud, GitBranch, Server, Sparkles, Terminal } from "./icons";
import type { Snapshot } from "./types";

const AGENTS = new Set(["codex", "claude", "opencode", "gemini", "cline", "copilot"]);

const HINT: Record<string, string> = {
  shell: "shell del workspace",
  ssh: "host de ~/.ssh/config",
  tailscale: "máquina Tailscale",
  nvim: "editor",
  lazygit: "git",
  btop: "monitor",
  yazi: "archivos",
  codex: "worktree + contexto",
  claude: "worktree + contexto",
  opencode: "worktree + contexto",
  gemini: "worktree + contexto",
  cline: "worktree + contexto",
  copilot: "worktree + contexto",
};

type Row = { kind: string; label: string; available: boolean };

export function NewTabPicker({
  snap,
  onLaunch,
  onSetDefault,
}: {
  snap: Snapshot;
  onLaunch: (kind: string) => void;
  onSetDefault: (kind: string) => void;
}) {
  const clis = snap.run_clis ?? [];
  const tools: Row[] = clis
    .filter((cli) => !AGENTS.has(cli.name) && cli.available)
    .map((cli) => ({ kind: cli.name, label: cli.name, available: cli.available }));
  const agents: Row[] = clis
    .filter((cli) => AGENTS.has(cli.name) && cli.available)
    .map((cli) => ({ kind: cli.name, label: cli.name, available: cli.available }));
  const core: Row[] = [
    { kind: "shell", label: "Terminal", available: true },
    { kind: "ssh", label: "SSH", available: true },
    { kind: "tailscale", label: "Tailscale", available: true },
  ];

  return (
    <div
      className="new-tab-menu"
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        if (e.key === "Escape") e.currentTarget.parentElement?.querySelector("button")?.blur();
      }}
    >
      <p className="new-tab-hint">la estrella es Ctrl-Alt-N. Un agente abre en git worktree y ve $LOLTERM_CONTEXT.</p>
      <Section title="Sesión" rows={core} snap={snap} onLaunch={onLaunch} onSetDefault={onSetDefault} />
      <Section title="Herramientas" rows={tools} snap={snap} onLaunch={onLaunch} onSetDefault={onSetDefault} />
      <Section title="Agentes" rows={agents} snap={snap} onLaunch={onLaunch} onSetDefault={onSetDefault} />
    </div>
  );
}

function Section({
  title,
  rows,
  snap,
  onLaunch,
  onSetDefault,
}: {
  title: string;
  rows: Row[];
  snap: Snapshot;
  onLaunch: (kind: string) => void;
  onSetDefault: (kind: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <section>
      <h3>{title}</h3>
      {rows.map((row) => {
        const on = snap.new_tab === row.kind;
        return (
          <div key={row.kind} className="new-tab-row">
            <button
              type="button"
              className="new-tab-hit"
              disabled={!row.available}
              title={row.available ? HINT[row.kind] : `${row.kind} no está en PATH`}
              onClick={() => onLaunch(row.kind)}
            >
              <KindIcon kind={row.kind} />
              <span>{row.label}</span>
              <span className="new-tab-meta">{row.available ? HINT[row.kind] : "no en PATH"}</span>
            </button>
            <button
              type="button"
              className={on ? "new-tab-star on" : "new-tab-star"}
              title={on ? "default de Ctrl-Alt-N" : "usar con Ctrl-Alt-N"}
              onClick={() => onSetDefault(row.kind)}
            >
              <Star filled={on} />
            </button>
          </div>
        );
      })}
    </section>
  );
}

function KindIcon({ kind }: { kind: string }) {
  if (kind === "ssh") return <Server size={13} />;
  if (kind === "tailscale") return <Cloud size={13} />;
  if (kind === "lazygit") return <GitBranch size={13} />;
  if (AGENTS.has(kind)) return <Sparkles size={13} />;
  return <Terminal size={13} />;
}

function Star({ filled }: { filled: boolean }) {
  return (
    <svg
      width={12}
      height={12}
      viewBox="0 0 24 24"
      fill={filled ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
    </svg>
  );
}
