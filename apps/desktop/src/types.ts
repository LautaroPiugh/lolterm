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
};

export type TabSnap = {
  name: string;
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
};

export type GitStatus = {
  branch: string;
  staged: number;
  unstaged: number;
  untracked: number;
  ahead: number;
  behind: number;
};

export type RunCli = { name: string; available: boolean };

export type Snapshot = {
  root: string;
  name: string;
  branch: string | null;
  active_tab: number;
  tabs: TabSnap[];
  git: GitStatus | null;
  git_log: string[];
  tree: TreeRow[];
  tailscale: unknown;
  run_clis: RunCli[];
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
  meta: { stack: string[]; git_remote: string | null; notes: string };
  machines: { name: string; target: string; user: string | null; kind: string }[];
};

export type CommandHit = { id: string; slash: string; hint: string };
export type HostItem = { name: string; target: string; hint: string };
export type Peer = { name: string; target: string; online: boolean; ip: string | null };

declare global {
  interface Window {
    lolterm: {
      invoke: (method: string, params?: unknown) => Promise<unknown>;
      onEvent: (cb: (msg: { event?: string; params?: { pane?: number; b64?: string } }) => void) => () => void;
      onChord: (cb: (chord: string) => void) => () => void;
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
