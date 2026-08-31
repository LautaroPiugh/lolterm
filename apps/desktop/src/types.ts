export type LayoutNode =
  | { type: "leaf"; pane: number }
  | {
      type: "split";
      dir: "columns" | "rows";
      percent: number;
      first: LayoutNode;
      second: LayoutNode;
    };

export type PaneSnap = {
  id: number;
  title: string;
  program: string | null;
  remote: string | null;
  worktree?: string | null;
};

export type TabSnap = {
  name: string;
  kind?: string;
  rel?: string | null;
  focused: number;
  zoomed: number | null;
  layout: LayoutNode;
  panes: PaneSnap[];
};

export type TreeRow = {
  rel: string;
  name: string;
  depth: number;
  is_dir: boolean;
  expanded: boolean;
  mark: string | null;
  lang: string | null;
  hidden?: boolean;
};

export type GitFile = {
  path: string;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
  mark: string;
};

export type GitStatus = {
  branch: string;
  staged: number;
  unstaged: number;
  untracked: number;
  ahead: number;
  behind: number;
};

export type GitWorktree = {
  path: string;
  branch: string | null;
  commit: string;
  detached: boolean;
  current: boolean;
  dirty: boolean;
};

export type RunCli = { name: string; available: boolean; version?: string | null };

export type Snapshot = {
  root: string;
  name: string;
  branch: string | null;
  active_tab: number;
  tabs: TabSnap[];
  git: GitStatus | null;
  git_files?: GitFile[];
  git_branches?: string[];
  git_log: string[];
  git_worktrees?: GitWorktree[];
  tree: TreeRow[];
  tailscale: unknown;
  run_clis: RunCli[];
  tools?: {
    name: string;
    kind?: "agent" | "cli";
    hint?: string;
    available: boolean;
    version?: string | null;
    install: string;
  }[];
  agent_tools?: {
    name: string;
    kind?: "agent" | "cli";
    hint?: string;
    available: boolean;
    version?: string | null;
    install: string;
  }[];
  http?: { enabled: boolean; bind: string };
  notice: string | null;
  theme: string;
  ssh_user: string | null;
  ssh_tmux: string;
  ssh_tmux_session: string;
  keybindings: { chord: string; command: string }[];
  version: string;
  presets: { id: string; name: string; hint: string }[];
  workspaces: { name: string; root: string; root_label?: string; current: boolean }[];
  startup: { program: string; args: string[] }[];
  env: { key: string; value: string }[];
  /** Nombres de API keys guardadas (nunca los valores). */
  api_keys?: string[];
  meta: { stack: string[]; git_remote: string | null; notes: string };
  machines: { name: string; target: string; user: string | null; kind: string }[];
  new_tab: string;
  agent_worktrees?: boolean;
  agents?: { program: string; tab: number; tab_name: string; worktree?: string | null; focused: boolean; attention?: boolean }[];
  agent_log?: { ts: number; workspace: string; program: string; worktree?: string | null }[];
  installs?: { pane: number; tool: string; command: string; state: string; exit_code?: number | null; output: string }[];
  themes?: { id: string; label: string; hint: string; vars: Record<string, string> }[];
  extensions?: string[];
  status_ext?: { id: string; text: string }[];
  ext_commands?: { id: string; slash: string; hint: string; run: string; args?: string[] }[];
  commands_path?: string;
  keybindings_path?: string;
  held_panes?: number[];
  /** false = splash; true = git/árbol/CLIs listos. Si falta, se trata como listo. */
  booted?: boolean;
};

export type QuotaBar = {
  key: string;
  label: string;
  percent: number;
  reset?: string | null;
};

export type QuotaAgent = {
  id: string;
  label: string;
  available: boolean;
  running: boolean;
  pending?: boolean;
  supported?: boolean;
  note?: string | null;
  bars: QuotaBar[];
};

export type NowPlaying = {
  playing: boolean;
  artist: string;
  title: string;
  volume: number;
  player?: string;
  source?: string;
  art?: string | null;
};

export type Hud = {
  playerctl: boolean;
  sink?: boolean;
  volume?: number;
  music: NowPlaying | null;
  quota: QuotaAgent[];
  host?: { load?: string | null; mem?: number | null } | null;
  extra?: {
    disk?: number | null;
    battery?: number | null;
    ports?: { port: number; pid: number; pane: number; program: string }[];
    processes?: { pid: number; program: string; pane: number }[];
  };
  notice?: string | null;
};

export type CommandHit = { id: string; slash: string; hint: string };
export type HostItem = { name: string; target: string; hint: string };
export type Peer = { name: string; target: string; online: boolean; ip: string | null };

declare global {
  interface Window {
    lolterm: {
      invoke: (method: string, params?: unknown) => Promise<unknown>;
      onEvent: (cb: (msg: { event?: string; params?: { pane?: number; b64?: string; error?: string } }) => void) => () => void;
      onChord: (cb: (chord: string) => void) => () => void;
      openExternal: (url: string) => Promise<void>;
      openFolder: () => Promise<Snapshot | null>;
      window: {
        minimize: () => Promise<void>;
        maximize: () => Promise<void>;
        close: () => Promise<void>;
      };
      clipboard: {
        read: () => Promise<string>;
        write: (text: string) => Promise<void>;
      };
      update: {
        check: () => Promise<{
          available: boolean;
          current?: string;
          latest?: string;
          notes?: string;
          reason?: string;
          packageType?: "deb" | "rpm";
        }>;
        install: () => Promise<{ ok: boolean; version?: string; method?: string }>;
        relaunch: () => Promise<void>;
      };
    };
  }
}

export function b64encode(bytes: Uint8Array): string {
  let bin = "";
  bytes.forEach((b) => {
    bin += String.fromCharCode(b);
  });
  return btoa(bin);
}

export function b64decode(text: string): Uint8Array {
  const bin = atob(text);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
