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
import { applyXtermTheme, disposeTerm, refitAllTerminals, retainPanes, setPaneTitleHandler } from "./TerminalPane";
import { Welcome } from "./Welcome";
import { NewTabPicker } from "./NewTabPicker";
import { CommandsEditor } from "./CommandsEditor";
import { MediaDock, QuotaButton } from "./Hud";
import { applyDocumentTheme, isBuiltinTheme, swatchGradient, THEMES } from "./themes";
import { displayVersion, eraLabel } from "./version";
import { bindingFor, commandForChord, isChromeField, setBindings } from "./chords";
import type { CommandHit, HostItem, Hud, Peer, Snapshot, TabSnap, TreeRow } from "./types";

type Activity = "home" | "files" | "git" | "run" | "remote";
type Modal =
  | { kind: "palette"; query: string }
  | { kind: "run" }
  | { kind: "files"; query: string }
  | { kind: "ssh"; query: string }
  | { kind: "ts"; user: string; selected: number }
  | { kind: "theme" }
  | { kind: "commands" }
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

function tabRemote(tab: TabSnap): string | null {
  const focused = tab.panes.find((pane) => pane.id === tab.focused);
  return focused?.remote ?? tab.panes.find((pane) => pane.remote)?.remote ?? null;
}

function tabIcon(tab: TabSnap): IconFn {
  if (tabRemote(tab)) return Cloud;
  const key = `${tab.name} ${tab.panes[0]?.program ?? ""}`.toLowerCase();
  if (key.includes("nvim") || key.includes("vim")) return FileCode;
  if (key.includes("claude") || key.includes("codex") || key.includes("opencode") || key.includes("cline")) return Sparkles;
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

type DockEdge = "left" | "right" | "up" | "down";

function dockEdgeFromPoint(host: HTMLElement, clientX: number, clientY: number): DockEdge | null {
  const rect = host.getBoundingClientRect();
  if (rect.width < 8 || rect.height < 8) return null;
  const px = (clientX - rect.left) / rect.width;
  const py = (clientY - rect.top) / rect.height;
  const dist = { left: px, right: 1 - px, up: py, down: 1 - py };
  let edge: DockEdge = "left";
  let min = dist.left;
  for (const key of ["right", "up", "down"] as const) {
    if (dist[key] < min) {
      min = dist[key];
      edge = key;
    }
  }
  if (min > 0.25) return null;
  return edge;
}

function ThemePicker({
  current,
  themes,
  onPick,
}: {
  current: string;
  themes: { id: string; label: string; hint: string }[];
  onPick: (id: string) => void;
}) {
  return (
    <div className="theme-list">
      {themes.map((item) => (
        <button
          key={item.id}
          type="button"
          className={current === item.id ? "theme-card on" : "theme-card"}
          onClick={() => onPick(item.id)}
        >
          <span className="theme-swatch" style={{ background: swatchGradient(item.id) }} />
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
  const [bootErr, setBootErr] = useState<string | null>(null);
  const [banner, setBanner] = useState<string | null>(null);
  const [update, setUpdate] = useState<
    | null
    | { kind: "available"; latest: string }
    | { kind: "busy"; label: string }
    | { kind: "done"; latest: string; method: string }
    | { kind: "error"; error: string }
  >(null);
  const [sshUser, setSshUser] = useState("");
  const [renaming, setRenaming] = useState<number | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [renameWs, setRenameWs] = useState(false);
  const [gearOpen, setGearOpen] = useState(false);
  const [quotaOpen, setQuotaOpen] = useState(false);
  const [mediaOpen, setMediaOpen] = useState(false);
  const [hud, setHud] = useState<Hud | null>(null);
  const [newTabOpen, setNewTabOpen] = useState(false);
  const [envKey, setEnvKey] = useState("");
  const [envVal, setEnvVal] = useState("");
  const [wsNotes, setWsNotes] = useState("");
  const [sshDest, setSshDest] = useState("");
  const [tmuxSession, setTmuxSession] = useState("lolterm");
  const [draggingTab, setDraggingTab] = useState<number | null>(null);
  const [dockEdge, setDockEdge] = useState<DockEdge | null>(null);
  const gearRef = useRef<HTMLDivElement>(null);
  const quotaRef = useRef<HTMLDivElement>(null);
  const mediaRef = useRef<HTMLDivElement>(null);
  const newTabRef = useRef<HTMLDivElement>(null);
  const hudHoldUntil = useRef(0);

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

  const musicAction = useCallback((action: string, volume?: number) => {
    hudHoldUntil.current = Date.now() + 900;
    setHud((prev) => {
      if (!prev) return prev;
      if (action === "playPause" && prev.music) {
        return { ...prev, music: { ...prev.music, playing: !prev.music.playing } };
      }
      if (action === "volume" && volume != null) {
        return { ...prev, volume };
      }
      return prev;
    });
    void window.lolterm.invoke("music", { action, volume }).then((next) => {
      if (!next || typeof next !== "object") return;
      setHud(next as Hud);
    });
  }, []);

  const launchKind = useCallback(
    async (kind: string) => {
      setNewTabOpen(false);
      if (kind === "shell") {
        await call("newTab");
        return;
      }
      if (kind === "ssh") {
        setModal({ kind: "ssh", query: "" });
        return;
      }
      if (kind === "tailscale") {
        setModal({ kind: "ts", user: sshUser, selected: 0 });
        return;
      }
      await call("newTab", { program: kind });
    },
    [call, sshUser],
  );

  const runBound = useCallback(
    async (name: string) => {
      const key = name.trim().replace(/^\//, "");
      if (key === "tab.new" || key === "tab-new") {
        await launchKind(snap?.new_tab || "shell");
        return;
      }
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
      if (key === "ui.commands" || key === "commands") {
        setModal({ kind: "commands" });
        return;
      }
      if (key === "ui.quota" || key === "quota") {
        setQuotaOpen((open) => !open);
        return;
      }
      if (key === "ui.media" || key === "media") {
        setMediaOpen((open) => !open);
        return;
      }
      if (key === "music.playPause" || key === "play-pause") {
        musicAction("playPause");
        return;
      }
      if (key === "music.next" || key === "music-next") {
        musicAction("next");
        return;
      }
      if (key === "music.prev" || key === "music-prev") {
        musicAction("prev");
        return;
      }
      if (key === "app.update" || key === "update") {
        setUpdate({ kind: "busy", label: "buscando actualización…" });
        try {
          const info = await window.lolterm.update.check();
          if (info.available && info.latest) {
            setUpdate({ kind: "available", latest: info.latest });
          } else {
            setUpdate(null);
            if (info.reason === "github-404") {
              setBanner("GitHub 404: el repo es privado o no hay release latest. En dev se usa `gh auth`; el .deb público necesita el repo público.");
            } else if (info.reason === "github-403") {
              setBanner("GitHub 403: rate limit o sin permiso para leer releases.");
            } else if (info.reason === "no-deb") {
              setBanner(`v${info.latest} no trae .deb + SHA256SUMS.txt`);
            } else if (info.current) {
              setBanner(`ya estás en v${info.current}`);
            } else {
              setBanner("no hay .deb nuevo en GitHub");
            }
          }
        } catch (err) {
          setUpdate({ kind: "error", error: err instanceof Error ? err.message : String(err) });
        }
        return;
      }
      await call("dispatch", { id: key });
    },
    [call, launchKind, musicAction, snap?.active_tab, snap?.new_tab, snap?.tabs, sshUser],
  );

  useEffect(() => {
    setPaneTitleHandler((pane, title) => {
      void call("setPaneTitle", { pane, title });
    });
    return () => setPaneTitleHandler(undefined);
  }, [call]);

  useEffect(() => {
    const off = window.lolterm.onEvent((msg) => {
      if (msg.event === "ready") {
        apply(msg.params);
        setBootErr(null);
        setBanner(null);
      }
      if (msg.event === "core-down") {
        setBanner(msg.params?.error ?? "lolterm-core se cayó");
      }
      if (msg.event === "core-error") {
        setBanner(msg.params?.error ?? "error del core");
      }
      if (msg.event === "exit" && msg.params?.pane != null) {
        disposeTerm(msg.params.pane);
        void call("snapshot");
      }
    });
    void call("snapshot").catch((err: unknown) => {
      setBootErr(err instanceof Error ? err.message : String(err));
    });
    void window.lolterm.invoke("projects").then((list) => setProjects((list as string[]) ?? []));
    const timer = window.setTimeout(() => {
      void window.lolterm.update
        .check()
        .then((info) => {
          if (info.available && info.latest) setUpdate({ kind: "available", latest: info.latest });
        })
        .catch(() => {});
    }, 4000);
    return () => {
      window.clearTimeout(timer);
      off();
    };
  }, [apply, call]);

  useEffect(() => {
    if (snap) return;
    const timer = window.setTimeout(() => {
      setBootErr((prev) => prev ?? "el core no respondió");
    }, 8000);
    return () => window.clearTimeout(timer);
  }, [snap]);

  useEffect(() => {
    if (!snap) return;
    const live = new Set<number>();
    for (const item of snap.tabs) {
      for (const pane of item.panes) live.add(pane.id);
    }
    retainPanes(live);
  }, [snap]);

  useEffect(() => {
    if (!snap) return;
    refitAllTerminals();
  }, [snap?.active_tab, snap?.root, snap?.tabs.length, snap?.tabs[snap.active_tab ?? 0]?.zoomed]);

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
    if (!quotaOpen) return;
    const onDown = (event: MouseEvent) => {
      if (quotaRef.current && !quotaRef.current.contains(event.target as Node)) {
        setQuotaOpen(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [quotaOpen]);

  useEffect(() => {
    if (!mediaOpen) return;
    const onDown = (event: MouseEvent) => {
      if (mediaRef.current && !mediaRef.current.contains(event.target as Node)) {
        setMediaOpen(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [mediaOpen]);

  useEffect(() => {
    let stop = false;
    const tick = () => {
      void window.lolterm.invoke("hud").then((value) => {
        if (stop || !value || typeof value !== "object") return;
        if (Date.now() < hudHoldUntil.current) return;
        const next = value as Hud;
        setHud(next);
      });
    };
    tick();
    const id = window.setInterval(tick, quotaOpen ? 400 : 900);
    return () => {
      stop = true;
      window.clearInterval(id);
    };
  }, [quotaOpen]);

  useEffect(() => {
    if (!newTabOpen) return;
    const onDown = (event: MouseEvent) => {
      if (newTabRef.current && !newTabRef.current.contains(event.target as Node)) {
        setNewTabOpen(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [newTabOpen]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (modal?.kind === "commands") return;
        setModal(null);
        setRenaming(null);
        setRenameWs(false);
        setGearOpen(false);
        setQuotaOpen(false);
        setMediaOpen(false);
        setNewTabOpen(false);
        return;
      }
      if (modal?.kind === "commands") {
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
  }, [modal?.kind, runBound]);

  useEffect(() => {
    return window.lolterm.onChord((chord) => {
      const command = commandForChord(chord);
      if (command) void runBound(command);
    });
  }, [runBound]);

  useEffect(() => {
    if (modal?.kind === "palette" || modal?.kind === "commands") {
      const query = modal.kind === "palette" ? modal.query : "";
      void window.lolterm.invoke("commands", { query }).then((list) => {
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
    if (snap?.ssh_tmux !== undefined) setTmuxSession(snap.ssh_tmux);
  }, [snap?.ssh_tmux]);

  useEffect(() => {
    if (!snap) return;
    const pack = (snap.themes ?? []).find((item) => item.id === snap.theme);
    const builtin = isBuiltinTheme(snap.theme);
    const keys = ["fill", "text", "brand", "bar", "pane", "muted", "focus", "border", "err", "ok"];
    if (builtin) {
      applyDocumentTheme(snap.theme);
      for (const key of keys) document.documentElement.style.removeProperty(`--${key}`);
    } else {
      document.documentElement.dataset.theme = "custom";
      if (pack) {
        for (const [key, value] of Object.entries(pack.vars ?? {})) {
          document.documentElement.style.setProperty(`--${key}`, value);
        }
      }
    }
    applyXtermTheme(snap.theme, builtin ? undefined : pack?.vars);
  }, [snap]);

  const tab = snap?.tabs[snap.active_tab];
  const remoteHost = tab ? tabRemote(tab) : null;
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
    return (
      <div className="boot">
        {bootErr ? `LoLTerm · no arrancó (${bootErr})` : "LoLTerm · abriendo PTY…"}
      </div>
    );
  }

  const gitAdds = (snap.git?.staged ?? 0) + (snap.git?.untracked ?? 0);
  const gitDels = snap.git?.unstaged ?? 0;

  return (
    <div className="shell">
      <header className="titlebar">
        <button type="button" className="titlebar-wordmark" onClick={() => setActivity("home")}>
          <img className="titlebar-icon" src={`${import.meta.env.BASE_URL}icon.png`} alt="" width={20} height={20} />
          <span className="lol">lol</span>
          <span className="mark">term</span>
          <span className="ver" title={`LoLTerm ${displayVersion(snap.version)} · ${eraLabel(snap.version)}`}>
            {displayVersion(snap.version)}
          </span>
        </button>
        <div className="titlebar-center">
          <button
            type="button"
            className="workspace-pill"
            title="Workspaces · clic abre Inicio · Ctrl-Alt-[ ] cicla"
            onClick={() => {
              if (activity === "home" && sidebar) {
                void runBound("workspace.next");
                return;
              }
              setActivity("home");
              setSidebar(true);
            }}
          >
            <GitBranch size={12} color="var(--brand)" />
            <span className="proj">{snap.name}</span>
            <span className="sep">:</span>
            <span className="branch">{snap.branch ?? "HEAD"}</span>
          </button>
        </div>
        <div className="titlebar-controls">
          <QuotaButton
            open={quotaOpen}
            agents={hud?.quota ?? []}
            onToggle={() => setQuotaOpen((open) => !open)}
            wrapRef={quotaRef}
          />
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
                <button
                  type="button"
                  className="gear-hit"
                  onClick={() => {
                    setGearOpen(false);
                    setModal({ kind: "commands" });
                  }}
                >
                  <Command size={12} color="var(--muted)" />
                  <span>Comandos y atajos</span>
                  <span className="hint">Ctrl-Alt-,</span>
                </button>
                <button
                  type="button"
                  className="gear-hit"
                  onClick={() => {
                    setGearOpen(false);
                    void runBound("app.update");
                  }}
                >
                  <Sparkles size={12} color="var(--muted)" />
                  <span>Buscar actualización</span>
                  <span className="hint">/update</span>
                </button>
                <details open>
                  <summary>Tema</summary>
                  <ThemePicker
                    current={snap.theme}
                    themes={snap.themes ?? THEMES}
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
      {update && (
        <div className="update-bar">
          {update.kind === "available" && (
            <>
              <span>
                LoLTerm <strong>v{update.latest}</strong> está en GitHub. Instala el <code>.deb</code> (Ubuntu)
                después de verificar SHA256.
              </span>
              <button
                type="button"
                className="update-btn"
                onClick={() => {
                  const latest = update.latest;
                  setUpdate({ kind: "busy", label: "descargando y verificando SHA256…" });
                  void window.lolterm.update
                    .install()
                    .then((result) => {
                      setUpdate({
                        kind: "done",
                        latest: result.version ?? latest,
                        method: result.method ?? "pkexec",
                      });
                    })
                    .catch((err: unknown) => {
                      setUpdate({ kind: "error", error: err instanceof Error ? err.message : String(err) });
                    });
                }}
              >
                Instalar
              </button>
              <button type="button" className="update-btn ghost" onClick={() => setUpdate(null)}>
                Después
              </button>
            </>
          )}
          {update.kind === "busy" && <span>{update.label}</span>}
          {update.kind === "done" && (
            <>
              <span>
                {update.method === "xdg-open"
                  ? `v${update.latest} listo en el instalador del sistema.`
                  : `v${update.latest} instalado. Reiniciá LoLTerm.`}
              </span>
              {update.method !== "xdg-open" && (
                <button type="button" className="update-btn" onClick={() => void window.lolterm.update.relaunch()}>
                  Reiniciar
                </button>
              )}
              <button type="button" className="update-btn ghost" onClick={() => setUpdate(null)}>
                Cerrar
              </button>
            </>
          )}
          {update.kind === "error" && (
            <>
              <span>{update.error}</span>
              <button type="button" className="update-btn ghost" onClick={() => setUpdate(null)}>
                Cerrar
              </button>
            </>
          )}
        </div>
      )}
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
                {(snap.startup?.length ?? 0) > 0 && (
                  <p className="workspace-startup-hint">
                    al abrir: {snap.startup.map((cmd) => cmd.program).join(" · ")}
                  </p>
                )}
                <p className="workspace-nav-hint">Ctrl-Alt-[ ] cicla workspaces</p>
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
                      className={row.hidden ? "tree-item hidden" : "tree-item"}
                      title={row.hidden ? `${row.name} (oculto)` : (row.lang ?? undefined)}
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
                  <input
                    value={tmuxSession}
                    placeholder="prefijo tmux (vacío = ssh directo)"
                    spellCheck={false}
                    autoComplete="off"
                    onChange={(e) => setTmuxSession(e.target.value)}
                    onBlur={() => {
                      if (tmuxSession !== (snap.ssh_tmux ?? "lolterm")) {
                        void call("setRemoteTmux", { tmux: tmuxSession });
                      }
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") e.currentTarget.blur();
                    }}
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
            <div className="tabs-scroll">
            {snap.tabs.map((item, index) => {
              const Icon = tabIcon(item);
              const on = index === snap.active_tab;
              const remote = tabRemote(item);
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
                  className={`${on ? "tab-pill on" : "tab-pill"}${remote ? " remote" : ""}`}
                  draggable
                  onClick={() => void call("selectTab", { index })}
                  onDoubleClick={(e) => {
                    e.stopPropagation();
                    setRenaming(index);
                    setRenameDraft(item.name);
                  }}
                  onDragStart={() => {
                    setDraggingTab(index);
                    setDockEdge(null);
                  }}
                  onDragEnd={() => {
                    setDraggingTab(null);
                    setDockEdge(null);
                  }}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => {
                    const from = draggingTab;
                    setDraggingTab(null);
                    setDockEdge(null);
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
            </div>
            <div className="new-tab-wrap" ref={newTabRef}>
              <button
                type="button"
                className={newTabOpen ? "tab-add on" : "tab-add"}
                title="Nueva tab"
                onClick={() => setNewTabOpen((open) => !open)}
              >
                <Plus size={14} />
              </button>
              {newTabOpen ? (
                <NewTabPicker
                  snap={snap}
                  onLaunch={(kind) => void launchKind(kind)}
                  onSetDefault={(kind) => void call("setNewTab", { kind })}
                />
              ) : null}
            </div>
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
          <div
            className="panes"
            onDragOver={(e) => {
              if (draggingTab == null || draggingTab === snap.active_tab) return;
              e.preventDefault();
              setDockEdge(dockEdgeFromPoint(e.currentTarget, e.clientX, e.clientY));
            }}
            onDragLeave={(e) => {
              if (!e.currentTarget.contains(e.relatedTarget as Node)) setDockEdge(null);
            }}
            onDrop={(e) => {
              e.preventDefault();
              const from = draggingTab;
              const edge = dockEdgeFromPoint(e.currentTarget, e.clientX, e.clientY);
              setDraggingTab(null);
              setDockEdge(null);
              if (from == null || edge == null || from === snap.active_tab) return;
              void call("dockTab", { from, edge }).then(() => refitAllTerminals());
            }}
          >
            {snap.tabs.length === 0 ? (
              <Welcome
                snap={snap}
                onNewTab={() => void launchKind(snap.new_tab || "shell")}
                onOpenFolder={() => void window.lolterm.openFolder().then(apply)}
                onPalette={() => setModal({ kind: "palette", query: "" })}
                onOpenWorkspace={(path) => void call("openProject", { path })}
                onRun={(program) => void call("run", { program, args: [] })}
                onFiles={() => setModal({ kind: "files", query: "" })}
                onSsh={() => setModal({ kind: "ssh", query: "" })}
                onTs={() => setModal({ kind: "ts", user: sshUser, selected: 0 })}
                onConnectMachine={(target) =>
                  void call("connectMachine", { target, user: sshUser.trim() || undefined })
                }
                onPreset={(id) => void call("applyPreset", { id })}
              />
            ) : (
              tab && (
                <SplitView
                  node={tab.layout}
                  panes={tab.panes}
                  focused={tab.focused}
                  zoomed={tab.zoomed}
                  onFocus={(id) => void call("focus", { pane: id })}
                />
              )
            )}
            {dockEdge && draggingTab != null && draggingTab !== snap.active_tab && (
              <div className={`dock-overlay ${dockEdge}`} />
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
          <GitBranch size={11} />
          {snap.git?.branch ?? "—"}
        </span>
        <span className="status-sep">·</span>
        <span className="status-path">{snap.root}</span>
        {hud?.host?.load && (
          <>
            <span className="status-sep">·</span>
            <span className="status-item" title="load average">
              {hud.host.load}
              {hud.host.mem != null ? ` · ${hud.host.mem}% ram` : ""}
            </span>
          </>
        )}
        {remoteHost && (
          <>
            <span className="status-sep">·</span>
            <span className="status-item status-remote">
              <Cloud size={11} />
              {remoteHost}
              {snap.ssh_tmux_session ? ` · ${snap.ssh_tmux_session}` : ""}
            </span>
          </>
        )}
        {(snap.agents ?? []).length > 0 && (
          <>
            <span className="status-sep">·</span>
            <span className="status-item" title={(snap.agents ?? []).map((a) => a.worktree ?? a.program).join(" · ")}>
              <Sparkles size={11} />
              {(snap.agents ?? []).map((a) => a.program).join(" · ")}
            </span>
          </>
        )}
        {(snap.status_ext ?? []).map((item) => (
          <span key={item.id} className="status-item" title={item.id}>
            {item.text}
          </span>
        ))}
        <div className="status-media-wrap" ref={mediaRef}>
          <button
            type="button"
            className="status-media"
            title={
              hud?.music
                ? `${hud.music.source ?? "media"} · ${hud.music.title}`
                : hud?.playerctl
                  ? "media (playerctl · YouTube/Spotify Web/MPRIS)"
                  : "instalá playerctl"
            }
            onClick={() => setMediaOpen((open) => !open)}
          >
            {hud?.music ? `${hud.music.playing ? "▶ " : ""}${hud.music.title}` : "media"}
          </button>
          {mediaOpen && <MediaDock hud={hud} open={mediaOpen} onClose={() => setMediaOpen(false)} onAction={musicAction} />}
        </div>
        <span className="status-shortcut">Ctrl+B paleta · Ctrl+Alt+[ ] workspaces · clic en el nombre</span>
        {banner && <span className="notice">{banner}</span>}
      </footer>

      {modal?.kind === "palette" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="cmd-palette" onClick={(e) => e.stopPropagation()}>
            <div className="cmd-palette-input-row">
              <Command size={13} color="var(--brand)" />
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
      {modal?.kind === "commands" && (
        <div className="modal" onClick={() => setModal(null)}>
          <CommandsEditor
            commands={snap.ext_commands ?? []}
            bindings={snap.keybindings ?? []}
            catalog={cmds}
            commandsPath={snap.commands_path ?? "~/.config/lolterm/commands.toml"}
            keybindingsPath={snap.keybindings_path ?? "~/.config/lolterm/keybindings.toml"}
            onSave={(draft) => void call("saveExtCommand", draft)}
            onRemove={(id) => void call("removeExtCommand", { id })}
            onBind={(chord, command) => void call("setKeybinding", { chord, command })}
            onResetKeys={() => void call("resetKeybindings")}
            onOpenFile={(file) => {
              setModal(null);
              void call("openConfig", { file });
            }}
            onClose={() => setModal(null)}
          />
        </div>
      )}
      {modal?.kind === "theme" && (
        <div className="modal" onClick={() => setModal(null)}>
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <h2>tema</h2>
            <ThemePicker
              current={snap.theme}
              themes={snap.themes ?? THEMES}
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
