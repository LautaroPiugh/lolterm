import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from "react";
import { SplitView } from "./SplitView";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Cloud,
  Columns,
  Command,
  Copy,
  FileCode,
  Files,
  FolderPlus,
  GitBranch,
  GitCommitHorizontal,
  Home,
  Minus,
  Network,
  Plus,
  Rows,
  Server,
  Sparkles,
  Square,
  Terminal,
  X,
  Settings,
} from "./icons";
import { FileTypeIcon, FolderTypeIcon } from "./fileIcons";
import { applyXtermTheme, disposeTerm, retainPanes } from "./TerminalPane";
import { THEMES, parseTheme, type ThemeId } from "./themes";
import { displayVersion, eraLabel } from "./version";
import { bindingFor, isChromeField, setBindings } from "./chords";
import type { CommandHit, HostItem, Peer, Snapshot, TabSnap, TreeRow } from "./types";

type Activity = "home" | "files" | "git" | "run" | "remote";
type Modal =
  | { kind: "palette"; query: string }
  | { kind: "run" }
  | { kind: "files"; query: string }
  | { kind: "ssh"; query: string }
  | { kind: "ts"; user: string; selected: number }
  | { kind: "theme" }
  | null;

type IconFn = ComponentType<{ size?: number; color?: string }>;

const RAIL: { id: Activity; Icon: IconFn; tip: string }[] = [
  { id: "home", Icon: Home, tip: "Inicio" },
  { id: "files", Icon: Files, tip: "Explorer" },
  { id: "git", Icon: GitBranch, tip: "Git" },
  { id: "run", Icon: Terminal, tip: "CLIs" },
  { id: "remote", Icon: Cloud, tip: "Remoto" },
];

const SIDE_LABEL: Record<Activity, string> = {
  home: "Inicio",
  files: "Explorer",
  git: "Git",
  run: "CLIs",
  remote: "Remoto",
};

function tabIcon(tab: TabSnap): IconFn {
  const key = `${tab.name} ${tab.panes[0]?.program ?? ""}`.toLowerCase();
  if (key.includes("nvim") || key.includes("vim")) return FileCode;
  if (key.includes("claude") || key.includes("codex")) return Sparkles;
  if (key.includes("lazygit") || key.includes("git")) return GitBranch;
  if (key.includes("ssh")) return Server;
  return Terminal;
}

function fileGlyph(row: TreeRow) {
  if (row.is_dir) return <FolderTypeIcon open={row.expanded} size={16} />;
  return <FileTypeIcon lang={row.lang} size={16} />;
}

function badgeClass(mark: string | null) {
  if (mark === "M") return "m";
  if (mark === "A") return "a";
  if (mark === "?") return "u";
  return "u";
}

function projectName(path: string) {
  const parts = path.replace(/\/$/, "").split("/");
  return parts[parts.length - 1] || path;
}

function ThemePicker({
  current,
  onPick,
}: {
  current: ThemeId;
  onPick: (id: ThemeId) => void;
}) {
  return (
    <div className="theme-list">
      {THEMES.map((item) => (
        <button
          key={item.id}
          type="button"
          className={current === item.id ? "theme-card on" : "theme-card"}
          onClick={() => onPick(item.id)}
        >
          <span className={`theme-swatch ${item.id}`} />
          <span>{item.label}</span>
          <span className="hint">{item.hint}</span>
        </button>
      ))}
    </div>
  );
}

export default function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [activity, setActivity] = useState<Activity>("home");
  const [sidebar, setSidebar] = useState(true);
  const [modal, setModal] = useState<Modal>(null);
  const [hosts, setHosts] = useState<HostItem[]>([]);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [projects, setProjects] = useState<string[]>([]);
  const [fileHits, setFileHits] = useState<{ rel: string }[]>([]);
  const [cmds, setCmds] = useState<CommandHit[]>([]);
  const [banner, setBanner] = useState<string | null>(null);
  const [sshUser, setSshUser] = useState("");
  const [renaming, setRenaming] = useState<number | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [renameWs, setRenameWs] = useState(false);
  const [gearOpen, setGearOpen] = useState(false);
  const [envKey, setEnvKey] = useState("");
  const [envVal, setEnvVal] = useState("");
  const [wsNotes, setWsNotes] = useState("");
  const [sshDest, setSshDest] = useState("");
  const dragTab = useRef<number | null>(null);
  const gearRef = useRef<HTMLDivElement>(null);

  const apply = useCallback((value: unknown) => {
    if (value && typeof value === "object" && "tabs" in value) {
      setSnap(value as Snapshot);
    }
  }, []);

  const call = useCallback(
    async (method: string, params?: unknown) => {
      const result = await window.lolterm.invoke(method, params);
      apply(result);
      return result;
    },
    [apply],
  );

  const runBound = useCallback(
    async (name: string) => {
      const key = name.trim().replace(/^\//, "");
      if (key === "ui.palette" || key === "palette") {
        setModal({ kind: "palette", query: "" });
        return;
      }
      if (key === "ui.run" || key === "run") {
        setModal({ kind: "run" });
        return;
      }
      if (key === "ui.files" || key === "files") {
        setModal({ kind: "files", query: "" });
        return;
      }
      if (key === "ui.ssh" || key === "ssh") {
        setModal({ kind: "ssh", query: "" });
        return;
      }
      if (key === "ui.tsSsh" || key === "ts-ssh") {
        setModal({ kind: "ts", user: sshUser, selected: 0 });
        return;
      }
      if (key === "ui.sidebar" || key === "sidebar") {
        setSidebar((v) => !v);
        return;
      }
      if (key === "ui.theme" || key === "theme") {
        setModal({ kind: "theme" });
        return;
      }
      if (key === "ui.tabRename" || key === "tab-rename") {
        const index = snap?.active_tab ?? 0;
        setRenaming(index);
        setRenameDraft(snap?.tabs[index]?.name ?? "");
        return;
      }
      await call("dispatch", { id: key });
    },
    [call, snap?.active_tab, snap?.tabs, sshUser],
  );

  useEffect(() => {
    const off = window.lolterm.onEvent((msg) => {
      if (msg.event === "ready") apply(msg.params);
      if (msg.event === "exit" && msg.params?.pane != null) {
        disposeTerm(msg.params.pane);
        void call("snapshot");
      }
    });
    void call("snapshot");
    void window.lolterm.invoke("projects").then((list) => setProjects((list as string[]) ?? []));
    return off;
  }, [apply, call]);

  useEffect(() => {
    if (!snap) return;
    const live = new Set<number>();
    for (const item of snap.tabs) {
      for (const pane of item.panes) live.add(pane.id);
    }
    retainPanes(live);
  }, [snap]);

  useEffect(() => {
    setBindings(snap?.keybindings);
  }, [snap?.keybindings]);

  useEffect(() => {
    if (snap) setWsNotes(snap.meta?.notes ?? "");
  }, [snap?.meta?.notes]);

  useEffect(() => {
    if (!gearOpen) return;
    const onDown = (event: MouseEvent) => {
      if (gearRef.current && !gearRef.current.contains(event.target as Node)) {
        setGearOpen(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [gearOpen]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setModal(null);
        setRenaming(null);
        setRenameWs(false);
        setGearOpen(false);
        return;
      }
      if (isChromeField(e.target)) return;
      const hit = bindingFor(e);
      if (!hit) return;
      e.preventDefault();
      e.stopPropagation();
      void runBound(hit.command);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [runBound]);

  useEffect(() => {
    if (modal?.kind === "palette") {
      void window.lolterm.invoke("commands", { query: modal.query }).then((list) => {
        setCmds((list as CommandHit[]) ?? []);
      });
    }
    if (modal?.kind === "files") {
      void window.lolterm.invoke("searchFiles", { query: modal.query }).then((list) => {
        setFileHits((list as { rel: string }[]) ?? []);
      });
    }
    if (modal?.kind === "ssh") {
      void window.lolterm.invoke("sshHosts").then((list) => setHosts((list as HostItem[]) ?? []));
    }
    if (modal?.kind === "ts") {
      void window.lolterm.invoke("tsPeers").then((list) => setPeers((list as Peer[]) ?? []));
    }
  }, [modal]);

  useEffect(() => {
    if (activity !== "remote") return;
    void window.lolterm.invoke("tsPeers").then((list) => setPeers((list as Peer[]) ?? []));
    void window.lolterm.invoke("sshHosts").then((list) => setHosts((list as HostItem[]) ?? []));
  }, [activity]);

  useEffect(() => {
    if (!snap?.notice) return;
    setBanner(snap.notice);
    const timer = window.setTimeout(() => setBanner(null), 3000);
    return () => window.clearTimeout(timer);
  }, [snap?.notice]);

  useEffect(() => {
    if (snap?.ssh_user) setSshUser((prev) => prev || snap.ssh_user || "");
  }, [snap?.ssh_user]);

  useEffect(() => {
    const id = parseTheme(snap?.theme);
    document.documentElement.dataset.theme = id;
    applyXtermTheme(id);
  }, [snap?.theme]);

  const tab = snap?.tabs[snap.active_tab];
  const crumbs = useMemo(() => {
    if (!snap) return ["lolterm"];
    return snap.branch ? [snap.name, snap.branch] : [snap.name];
  }, [snap]);

  async function runCommand(slash: string) {
    setModal(null);
    await runBound(slash);
  }

  async function connectTs(target: string, user = sshUser) {
    const trimmed = user.trim();
    if (!trimmed) {
      setBanner("hace falta un usuario ssh");
      return;
    }
    setModal(null);
    await call("tsSsh", { target, user: trimmed });
  }

  function sshDestWithUser(host: string) {
    const user = sshUser.trim();
    if (!user || host.includes("@")) return host;
    return `${user}@${host}`;
  }

  if (!snap) {
    return <div className="boot">LoLTerm · abriendo PTY…</div>;
  }

  const gitAdds = (snap.git?.staged ?? 0) + (snap.git?.untracked ?? 0);
  const gitDels = snap.git?.unstaged ?? 0;

  return (
    <div className="shell">
      <header className="titlebar">
        <button type="button" className="titlebar-wordmark" onClick={() => setActivity("home")}>
          <span className="lol">lol</span>
          <span className="mark">term</span>
          <span className="ver" title={`LoLTerm ${displayVersion(snap.version)} · ${eraLabel(snap.version)}`}>
            {displayVersion(snap.version)}
          </span>
        </button>
        <div className="titlebar-center">
          <div className="workspace-pill">
            <GitBranch size={12} color="var(--brand)" />
            <span className="proj">{snap.name}</span>
            <span className="sep">:</span>
            <span className="branch">{snap.branch ?? "HEAD"}</span>
          </div>
        </div>
        <div className="titlebar-controls">
          <div className="gear-wrap" ref={gearRef}>
            <button
              type="button"
              className={gearOpen ? "wm-btn on" : "wm-btn"}
              title="Workspace"
              onClick={() => setGearOpen((open) => !open)}
            >
              <Settings size={12} />
            </button>
            {gearOpen && (
              <div className="gear-menu">
                <details open>
                  <summary>Tema</summary>
                  <ThemePicker
                    current={parseTheme(snap.theme)}
                    onPick={(id) => void call("setTheme", { theme: id })}
                  />
                </details>
                <details>
                  <summary>Layouts</summary>
                  {(snap.presets ?? []).map((preset) => (
                    <button
                      key={preset.id}
                      type="button"
                      className="gear-hit"
                      onClick={() => void call("applyPreset", { id: preset.id })}
                    >
                      <Columns size={12} color="var(--muted)" />
                      <span>{preset.name}</span>
                      <span className="hint">{preset.hint}</span>
                    </button>
                  ))}
                </details>
                <details>
                  <summary>Al abrir</summary>
                  {(snap.startup ?? []).map((cmd) => (
                    <button
                      key={cmd.program}
                      type="button"
                      className="gear-hit on"
                      title="quitar"
                      onClick={() => void call("removeStartup", { program: cmd.program })}
                    >
                      {cmd.program}
                      <span className="hint">quitar</span>
                    </button>
                  ))}
                  {snap.run_clis
                    .filter(
                      (cli) =>
                        cli.available && !(snap.startup ?? []).some((cmd) => cmd.program === cli.name),
                    )
                    .map((cli) => (
                      <button
                        key={cli.name}
                        type="button"
                        className="gear-hit"
                        onClick={() => void call("addStartup", { program: cli.name, args: [] })}
                      >
                        + {cli.name}
                      </button>
                    ))}
                </details>
                <details>
                  <summary>Entorno</summary>
                  {(snap.env ?? []).map((item) => (
                    <button
                      key={item.key}
                      type="button"
                      className="gear-hit on"
                      title="quitar"
                      onClick={() => void call("removeEnv", { key: item.key })}
                    >
                      {item.key}
                      <span className="hint">quitar</span>
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
                    <input
                      value={envKey}
                      onChange={(event) => setEnvKey(event.target.value)}
                      placeholder="NOMBRE"
                      spellCheck={false}
                      autoComplete="off"
                    />
                    <input
                      value={envVal}
                      onChange={(event) => setEnvVal(event.target.value)}
                      placeholder="valor"
                      spellCheck={false}
                      autoComplete="off"
                    />
                    <button type="submit" className="open-folder-btn" disabled={!envKey.trim()}>
                      Guardar
                    </button>
                  </form>
                </details>
                <details>
                  <summary>Proyecto</summary>
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
                      void call("setNotes", { notes: wsNotes });
                    }}
                  >
                    <textarea
                      value={wsNotes}
                      onChange={(event) => setWsNotes(event.target.value)}
                      placeholder="nota (sin secretos)"
                      rows={2}
                      spellCheck={false}
                    />
                    <button
                      type="submit"
                      className="open-folder-btn"
                      disabled={wsNotes.trim() === (snap.meta?.notes ?? "")}
                    >
                      Guardar nota
                    </button>
                  </form>
                </details>
              </div>
            )}
          </div>
          <button type="button" className="wm-btn" title="Minimizar" onClick={() => void window.lolterm.window.minimize()}>
            <Minus size={12} />
          </button>
          <button type="button" className="wm-btn" title="Maximizar" onClick={() => void window.lolterm.window.maximize()}>
            <Square size={12} />
          </button>
          <button type="button" className="wm-btn close" title="Cerrar" onClick={() => void window.lolterm.window.close()}>
            <X size={12} />
          </button>
        </div>
      </header>
      <div className="body">
        <nav className="rail">
          {RAIL.map(({ id, Icon, tip }) => (
            <button
              key={id}
              type="button"
              title={tip}
              className={activity === id && sidebar ? "rail-item on" : "rail-item"}
              onClick={() => {
                if (activity === id) setSidebar((v) => !v);
                else {
                  setActivity(id);
                  setSidebar(true);
                }
              }}
            >
              <Icon size={15} />
            </button>
          ))}
        </nav>
        {sidebar && (
          <aside className="sidebar">
            <div className="sidebar-header">{SIDE_LABEL[activity]}</div>
            {activity === "home" && (
              <div className="sidebar-content">
                {(snap.workspaces?.length
                  ? snap.workspaces.map((ws) => ({
                      key: ws.root,
                      name: ws.name,
                      path: ws.root,
                      current: ws.current,
                    }))
                  : projects.map((p) => ({
                      key: p,
                      name: projectName(p),
                      path: p,
                      current: p === snap.root,
                    }))
                ).map((ws) => (
                  <div key={ws.key} className={ws.current ? "workspace-row on" : "workspace-row"}>
                    {renameWs && ws.current ? (
                      <input
                        className="tab-rename"
                        autoFocus
                        value={renameDraft}
                        onChange={(e) => setRenameDraft(e.target.value)}
                        onBlur={() => {
                          if (renameDraft.trim()) void call("renameWorkspace", { name: renameDraft.trim() });
                          setRenameWs(false);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") e.currentTarget.blur();
                          if (e.key === "Escape") setRenameWs(false);
                        }}
                      />
                    ) : (
                      <button
                        type="button"
                        className="recent-item"
                        onClick={() => void call("openProject", { path: ws.path })}
                        onDoubleClick={(e) => {
                          if (!ws.current) return;
                          e.stopPropagation();
                          setRenameDraft(ws.name);
                          setRenameWs(true);
                        }}
                      >
                        <span className="proj-name">{ws.name}</span>
                      </button>
                    )}
                    {!ws.current && (
                      <button
                        type="button"
                        className="workspace-forget"
                        title="quitar"
                        onClick={() => void call("forgetWorkspace", { path: ws.path })}
                      >
                        <X size={11} />
                      </button>
                    )}
                  </div>
                ))}
                <button type="button" className="open-folder-btn" onClick={() => void window.lolterm.openFolder().then(apply)}>
                  <FolderPlus size={12} />
                  Abrir…
                </button>
              </div>
            )}
            {activity === "files" && (
              <>
                <div className="sidebar-tabs">
                  <span className="stab on">Files</span>
                  <button type="button" className="stab" onClick={() => setModal({ kind: "files", query: "" })}>
                    Search
                  </button>
                </div>
                <div className="sidebar-content">
                  {snap.tree.map((row) => (
                    <button
                      key={row.rel || "/"}
                      type="button"
                      className="tree-item"
                      title={row.lang ?? undefined}
                      style={{ paddingLeft: 8 + row.depth * 16 }}
                      onClick={() =>
                        row.is_dir
                          ? void call("toggleExpand", { rel: row.rel })
                          : void call("openFile", { rel: row.rel })
                      }
                    >
                      {row.is_dir ? (
                        row.expanded ? (
                          <ChevronDown size={10} color="#6C8070" />
                        ) : (
                          <ChevronRight size={10} color="#6C8070" />
                        )
                      ) : (
                        <span style={{ width: 10, flexShrink: 0 }} />
                      )}
                      <span style={{ marginLeft: 4, flexShrink: 0 }}>{fileGlyph(row)}</span>
                      <span className="tree-name" style={{ marginLeft: 5 }}>
                        {row.name}
                      </span>
                      {row.mark && <span className={`tree-badge ${badgeClass(row.mark)}`}>{row.mark}</span>}
                    </button>
                  ))}
                </div>
              </>
            )}
            {activity === "git" && (
              <>
                <div className="git-branch-bar">
                  <GitBranch size={12} color="#6C8070" />
                  <span className="branch-name">{snap.git?.branch ?? "sin repo"}</span>
                  {snap.git && (
                    <>
                      <span className="diff-chip diff-add">+{gitAdds}</span>
                      <span className="diff-chip diff-del">−{gitDels}</span>
                    </>
                  )}
                  <button type="button" className="lazygit-btn" onClick={() => void call("run", { program: "lazygit", args: [] })}>
                    lazygit
                  </button>
                </div>
                <div className="sidebar-content">
                  <div className="section-label">Log</div>
                  {snap.git_log.map((line) => {
                    const sha = line.slice(0, 7);
                    const msg = line.slice(8);
                    return (
                      <div key={line} className="git-log-sidebar-item">
                        <GitCommitHorizontal size={11} color="#488C58" />
                        <span className="git-sha">{sha}</span>
                        <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {msg || line}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </>
            )}
            {activity === "run" && (
              <div className="sidebar-content" style={{ paddingTop: 8 }}>
                {snap.run_clis.map((cli) => (
                  <button
                    key={cli.name}
                    type="button"
                    className="cli-item"
                    disabled={!cli.available}
                    onClick={() => void call("run", { program: cli.name, args: [] })}
                  >
                    <span className={cli.available ? "cli-check on" : "cli-check"}>
                      {cli.available && <Check size={9} strokeWidth={3} color="#fff" />}
                    </span>
                    <span className="cli-name">{cli.name}</span>
                  </button>
                ))}
              </div>
            )}
            {activity === "remote" && (
              <div className="sidebar-content">
                <div className="section-label">Tailscale</div>
                <div className="env-form">
                  <input
                    value={sshUser}
                    placeholder="usuario"
                    spellCheck={false}
                    autoComplete="off"
                    onChange={(e) => setSshUser(e.target.value)}
                  />
                </div>
                {peers.length === 0 && (snap.machines ?? []).every((m) => m.kind !== "tailscale") && (
                  <div className="proj-path" style={{ padding: "0 12px 8px" }}>
                    ningún peer · ¿tailscale up?
                  </div>
                )}
                {(snap.machines ?? [])
                  .filter((machine) => machine.kind === "tailscale")
                  .map((machine) => {
                    const peer = peers.find((item) => item.target === machine.target);
                    return (
                      <div key={machine.target} className="workspace-row">
                        <button
                          type="button"
                          className="remote-item"
                          onClick={() => void connectTs(machine.target)}
                        >
                          <Network size={12} color="var(--muted)" />
                          <span className="ri-name">{machine.name}</span>
                          <span className={peer?.online ? "ri-dot on" : "ri-dot off"} />
                        </button>
                        <button
                          type="button"
                          className="workspace-forget"
                          title="quitar"
                          onClick={() => void call("forgetMachine", { target: machine.target })}
                        >
                          <X size={11} />
                        </button>
                      </div>
                    );
                  })}
                {peers
                  .filter(
                    (peer) =>
                      !(snap.machines ?? []).some(
                        (machine) => machine.kind === "tailscale" && machine.target === peer.target,
                      ),
                  )
                  .map((peer) => (
                    <button
                      key={peer.target}
                      type="button"
                      className="remote-item"
                      onClick={() => void connectTs(peer.target)}
                    >
                      <Network size={12} color="var(--muted)" />
                      <span className="ri-name">{peer.name}</span>
                      <span className={peer.online ? "ri-dot on" : "ri-dot off"} />
                    </button>
                  ))}
                <div className="section-label">SSH</div>
                <form
                  className="env-form"
                  onSubmit={(event) => {
                    event.preventDefault();
                    const dest = sshDestWithUser(sshDest.trim());
                    if (!dest) return;
                    void call("ssh", { dest }).then(() => setSshDest(""));
                  }}
                >
                  <input
                    value={sshDest}
                    onChange={(event) => setSshDest(event.target.value)}
                    placeholder="user@host o alias"
                    spellCheck={false}
                    autoComplete="off"
                  />
                </form>
                {(snap.machines ?? [])
                  .filter((machine) => machine.kind !== "tailscale")
                  .map((machine) => (
                    <div key={machine.target} className="workspace-row">
                      <button
                        type="button"
                        className="remote-item"
                        onClick={() =>
                          void call("connectMachine", {
                            target: machine.target,
                            user: sshUser.trim() || undefined,
                          })
                        }
                      >
                        <Server size={12} color="var(--muted)" />
                        <span className="ri-name">{machine.name}</span>
                      </button>
                      <button
                        type="button"
                        className="workspace-forget"
                        title="quitar"
                        onClick={() => void call("forgetMachine", { target: machine.target })}
                      >
                        <X size={11} />
                      </button>
                    </div>
                  ))}
                {hosts
                  .filter(
                    (host) =>
                      !(snap.machines ?? []).some(
                        (machine) =>
                          machine.kind !== "tailscale" &&
                          (machine.target === host.target || machine.name === host.name),
                      ),
                  )
                  .slice(0, 12)
                  .map((host) => (
                    <button
                      key={host.target}
                      type="button"
                      className="remote-item"
                      onClick={() => void call("ssh", { dest: sshDestWithUser(host.target) })}
                    >
                      <Server size={12} color="var(--muted)" />
                      <span className="ri-name">{host.name}</span>
                    </button>
                  ))}
              </div>
            )}
          </aside>
        )}
        <main className="editor">
          <div className="tabs">
            {snap.tabs.map((item, index) => {
              const Icon = tabIcon(item);
              const on = index === snap.active_tab;
              if (renaming === index) {
                return (
                  <input
                    key={`rename-${index}`}
                    className="tab-rename"
                    autoFocus
                    value={renameDraft}
                    onChange={(e) => setRenameDraft(e.target.value)}
                    onBlur={() => {
                      void call("renameTab", { index, name: renameDraft });
                      setRenaming(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.currentTarget.blur();
                      }
                      if (e.key === "Escape") setRenaming(null);
                    }}
                  />
                );
              }
              return (
                <button
                  key={`${item.name}-${index}`}
                  type="button"
                  className={on ? "tab-pill on" : "tab-pill"}
                  draggable
                  onClick={() => void call("selectTab", { index })}
                  onDoubleClick={(e) => {
                    e.stopPropagation();
                    setRenaming(index);
                    setRenameDraft(item.name);
                  }}
                  onDragStart={() => {
                    dragTab.current = index;
                  }}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => {
                    const from = dragTab.current;
                    dragTab.current = null;
                    if (from == null || from === index) return;
                    void call("moveTab", { from, to: index });
                  }}
                >
                  <Icon size={12} color={on ? "#488C58" : "#6C8070"} />
                  <span>{item.name}</span>
                  <span
                    className="tab-close"
                    onClick={(e) => {
                      e.stopPropagation();
                      void call("closeTab", { index });
                    }}
                  >
                    <X size={10} />
                  </span>
                </button>
              );
            })}
            <button type="button" className="tab-add" title="Nueva terminal" onClick={() => void call("newTab")}>
              <Plus size={14} />
            </button>
            <button
              type="button"
              className="tab-add"
              title="Duplicar tab (Ctrl+Alt+D)"
              onClick={() => void call("duplicateTab", { index: snap.active_tab })}
            >
              <Copy size={14} />
            </button>
            <button type="button" className="tab-add" title="Split vertical" onClick={() => void call("split", { dir: "columns" })}>
              <Columns size={14} />
            </button>
            <button type="button" className="tab-add" title="Split horizontal" onClick={() => void call("split", { dir: "rows" })}>
              <Rows size={14} />
            </button>
            <button
              type="button"
              className={tab?.zoomed != null ? "tab-add on" : "tab-add"}
              title="Zoom pane (Ctrl-Alt-z)"
              onClick={() => void call("zoom")}
            >
              <Square size={14} />
            </button>
          </div>
          <div className="crumbs">
            {crumbs.map((part, i) => (
              <span key={`${part}-${i}`} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                {i > 0 && <ChevronRight size={10} color="#C5D4C5" />}
                <span className={i === crumbs.length - 1 ? "current" : ""}>{part}</span>
              </span>
            ))}
          </div>
          <div className="panes">
            {tab && (
              <SplitView
                node={tab.layout}
                panes={tab.panes}
                focused={tab.focused}
                zoomed={tab.zoomed}
                onFocus={(id) => void call("focus", { pane: id })}
              />
            )}
          </div>
        </main>
      </div>
      <footer className="status">
        <span className="status-item status-version" title={eraLabel(snap.version)}>
          {displayVersion(snap.version)}
        </span>
        <span className="status-sep">·</span>
        <span className="status-item">
          <GitBranch size={11} color="rgba(255,255,255,0.8)" />
          {snap.git?.branch ?? "—"}
        </span>
        <span className="status-sep">·</span>
        <span className="status-path">{snap.root}</span>
        <span className="status-shortcut">Ctrl+B paleta · Ctrl+Alt+[ ] workspaces</span>
        {banner && <span className="notice">{banner}</span>}
      </footer>

      {modal?.kind === "palette" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="cmd-palette" onClick={(e) => e.stopPropagation()}>
            <div className="cmd-palette-input-row">
              <Command size={13} color="#488C58" />
              <input
                className="cmd-palette-input"
                autoFocus
                value={modal.query}
                placeholder="run, split, ws-…"
                onChange={(e) => setModal({ kind: "palette", query: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && cmds[0]) void runCommand(cmds[0].id);
                }}
              />
              <span className="cmd-result-hint">Ctrl-b</span>
            </div>
            <div className="cmd-section-label">Acciones</div>
            {cmds.map((cmd) => (
              <button key={cmd.id} type="button" className="cmd-result" onClick={() => void runCommand(cmd.id)}>
                <Terminal size={12} color="#6C8070" />
                <span className="cmd-result-label">
                  /{cmd.slash} · {cmd.hint}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
      {modal?.kind === "theme" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <h2>tema</h2>
            <ThemePicker
              current={parseTheme(snap.theme)}
              onPick={(id) => {
                setModal(null);
                void call("setTheme", { theme: id });
              }}
            />
          </div>
        </div>
      )}
      {modal?.kind === "run" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <h2>abrir CLI</h2>
            {snap.run_clis.map((cli) => (
              <button
                key={cli.name}
                type="button"
                className="row"
                disabled={!cli.available}
                onClick={() => {
                  setModal(null);
                  void call("run", { program: cli.name, args: [] });
                }}
              >
                {cli.name}
              </button>
            ))}
          </div>
        </div>
      )}
      {modal?.kind === "files" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <input
              autoFocus
              value={modal.query}
              placeholder="archivo…"
              onChange={(e) => setModal({ kind: "files", query: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === "Enter" && fileHits[0]) {
                  setModal(null);
                  void call("openFile", { rel: fileHits[0].rel });
                }
              }}
            />
            {fileHits.slice(0, 20).map((hit) => (
              <button
                key={hit.rel}
                type="button"
                className="row"
                onClick={() => {
                  setModal(null);
                  void call("openFile", { rel: hit.rel });
                }}
              >
                {hit.rel}
              </button>
            ))}
          </div>
        </div>
      )}
      {modal?.kind === "ssh" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <input
              autoFocus
              value={modal.query}
              placeholder="user@host"
              onChange={(e) => setModal({ kind: "ssh", query: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === "Enter" && modal.query) {
                  setModal(null);
                  void call("ssh", { dest: modal.query });
                }
              }}
            />
            {hosts.map((host) => (
              <button
                key={host.target}
                type="button"
                className="row"
                onClick={() => {
                  setModal(null);
                  void call("ssh", { dest: host.target });
                }}
              >
                {host.name} · {host.hint}
              </button>
            ))}
          </div>
        </div>
      )}
      {modal?.kind === "ts" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <input
              autoFocus
              value={modal.user}
              placeholder="usuario ssh"
              onChange={(e) => {
                setSshUser(e.target.value);
                setModal({ ...modal, user: e.target.value });
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  const peer = peers[modal.selected] ?? peers[0];
                  if (peer) void connectTs(peer.target, modal.user);
                }
              }}
            />
            {peers.length === 0 && <p>ningún peer de Tailscale</p>}
            {peers.map((peer, index) => (
              <button
                key={peer.target}
                type="button"
                className="row"
                onClick={() => void connectTs(peer.target, modal.user)}
              >
                {peer.online ? "●" : "○"} {peer.name}
                {index === 0 ? " · Enter" : ""}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
