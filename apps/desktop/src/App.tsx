import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType, lazy, Suspense } from "react";
import { SplitView } from "./SplitView";
import {
  Check,
  ChevronRight,
  Cloud,
  Columns,
  Command,
  Copy,
  FileCode,
  Files,
  FolderPlus,
  GitBranch,
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
  Settings as GearIcon,
} from "./icons";
import { applyXtermTheme, disposeTerm, refitAllTerminals, retainPanes, setPaneTitleHandler } from "./TerminalPane";
import { BootScreen } from "./BootScreen";
import { Welcome } from "./Welcome";
import { NewTabPicker } from "./NewTabPicker";
import { ExplorerPanel } from "./ExplorerPanel";
import { GitPanel } from "./GitPanel";
import { CommandsEditor } from "./CommandsEditor";
import { Settings, type SettingsTab } from "./Settings";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { RestClient } from "./RestClient";
import { TouchBar } from "./TouchBar";
import { StatusBar } from "./StatusBar";
import { applyDocumentTheme, isBuiltinTheme } from "./themes";
import { pushDiag, readDiag } from "./diagnostics";
import { displayVersion, eraLabel } from "./version";
import { bindingFor, commandForChord, isChromeField, setBindings } from "./chords";
import {
  copyOnSelectEnabled,
  dismissCopyOnSelectPrompt,
  setCopyOnSelect,
  subscribeCopied,
  subscribeCopyOnSelectAsk,
  takePendingCopy,
  writeClipboard,
} from "./copyOnSelect";
import type { CommandHit, HostItem, Hud, Peer, Snapshot, TabSnap } from "./types";

const FileEditor = lazy(() => import("./FileEditor").then((mod) => ({ default: mod.FileEditor })));

type Activity = "home" | "files" | "git" | "run" | "remote";
type Modal =
  | { kind: "palette"; query: string }
  | { kind: "run" }
  | { kind: "files"; query: string }
  | { kind: "ssh"; query: string }
  | { kind: "ts"; user: string; selected: number }
  | { kind: "settings"; tab: SettingsTab }
  | { kind: "commands" }
  | { kind: "diag" }
  | { kind: "copyOnSelect" }
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
  if (tab.kind === "file") return FileCode;
  if (tab.kind === "rest") return Network;
  if (tabRemote(tab)) return Cloud;
  const key = `${tab.name} ${tab.panes[0]?.program ?? ""}`.toLowerCase();
  if (key.includes("nvim") || key.includes("vim")) return FileCode;
  if (key.includes("claude") || key.includes("codex") || key.includes("opencode") || key.includes("hermes") || key.includes("goose") || key.includes("aider") || key.includes("crush") || key.includes("qwen") || key.includes("openhands") || key.includes("cline") || key.includes("copilot") || key.includes("agy") || key.includes("antigravity")) return Sparkles;
  if (key.includes("lazygit") || key.includes("git")) return GitBranch;
  if (key.includes("ssh")) return Server;
  return Terminal;
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
  const [diag, setDiag] = useState(readDiag);
  const [copiedFlash, setCopiedFlash] = useState(0);
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
  const [quotaOpen, setQuotaOpen] = useState(false);
  const [mediaOpen, setMediaOpen] = useState(false);
  const [agentsTick, setAgentsTick] = useState(0);
  const [hud, setHud] = useState<Hud | null>(null);
  const [newTabOpen, setNewTabOpen] = useState(false);
  const [sshDest, setSshDest] = useState("");
  const [tmuxSession, setTmuxSession] = useState("lolterm");
  const [draggingTab, setDraggingTab] = useState<number | null>(null);
  const [dockEdge, setDockEdge] = useState<DockEdge | null>(null);
  const quotaRef = useRef<HTMLDivElement>(null);
  const mediaRef = useRef<HTMLDivElement>(null);
  const newTabRef = useRef<HTMLDivElement>(null);
  const hudHoldUntil = useRef(0);
  const dirtyFiles = useRef<Record<string, boolean>>({});
  const [, setDirtyTick] = useState(0);

  const markFileDirty = useCallback((rel: string, dirty: boolean) => {
    const was = dirtyFiles.current[rel] === true;
    if (was === dirty) return;
    const next = { ...dirtyFiles.current };
    if (dirty) next[rel] = true;
    else delete next[rel];
    dirtyFiles.current = next;
    setDirtyTick((n) => n + 1);
  }, []);

  const noteDiag = useCallback((kind: "error" | "warn" | "info", source: string, message: string) => {
    setDiag(pushDiag(kind, source, message));
  }, []);

  const apply = useCallback((value: unknown) => {
    if (value && typeof value === "object" && "tabs" in value) {
      setSnap(value as Snapshot);
    }
  }, []);

  const call = useCallback(
    async (method: string, params?: unknown) => {
      try {
        const result = await window.lolterm.invoke(method, params);
        apply(result);
        return result;
      } catch (err: unknown) {
        if (method !== "hud") {
          noteDiag("error", method, err instanceof Error ? err.message : String(err));
        }
        throw err;
      }
    },
    [apply, noteDiag],
  );

  const closeTabAt = useCallback(
    async (index: number) => {
      const item = snap?.tabs[index];
      const rel = item?.rel;
      if (item?.kind === "file" && rel && dirtyFiles.current[rel]) {
        if (!window.confirm(`¿Cerrar ${item.name} sin guardar?`)) return;
      }
      if (rel) markFileDirty(rel, false);
      await call("closeTab", { index });
    },
    [call, markFileDirty, snap?.tabs],
  );

  const chooseCopyOnSelect = useCallback((on: boolean) => {
    setCopyOnSelect(on);
    const pending = takePendingCopy();
    if (on && pending) void writeClipboard(pending);
    setModal(null);
    setBanner(on ? "copiar al seleccionar: sí" : "copiar al seleccionar: no");
    window.setTimeout(() => setBanner(null), 2500);
  }, []);

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
      if (kind === "rest") {
        setModal({ kind: "files", query: ".http" });
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
      if (key === "ui.rest" || key === "rest") {
        setModal({ kind: "files", query: ".http" });
        return;
      }
      if (key === "git.commit" || key === "commit") {
        setActivity("git");
        setSidebar(true);
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
        setModal({ kind: "settings", tab: "look" });
        return;
      }
      if (key === "ui.settings" || key === "settings") {
        setModal({ kind: "settings", tab: "look" });
        return;
      }
      if (key === "terminal.copyOnSelect" || key === "copy-select") {
        setModal({ kind: "copyOnSelect" });
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
      if (key === "ui.attention" || key === "attention") {
        setAgentsTick((n) => n + 1);
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
              setBanner("GitHub no devolvió una release latest pública para buscar actualizaciones.");
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
      if (key === "tab.close" || key === "tab-close") {
        await closeTabAt(snap?.active_tab ?? 0);
        return;
      }
      await call("dispatch", { id: key });
    },
    [call, closeTabAt, launchKind, musicAction, snap?.active_tab, snap?.new_tab, snap?.tabs, sshUser],
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
        void window.lolterm.invoke("projects").then((list) => setProjects((list as string[]) ?? []));
      }
      if (msg.event === "core-down") {
        const text = msg.params?.error ?? "lolterm-core se cayó";
        const reconnect = /reconectando/i.test(text);
        setBanner(reconnect ? null : text);
        noteDiag(reconnect ? "info" : "error", "core", text);
      }
      if (msg.event === "core-error") {
        const text = msg.params?.error ?? "error del core";
        setBanner(text);
        noteDiag("error", "core", text);
      }
      if (msg.event === "exit" && msg.params?.pane != null) {
        disposeTerm(msg.params.pane);
        void call("snapshot");
      }
    });
    void call("snapshot")
      .then(() => setBootErr(null))
      .catch((err: unknown) => {
        const text = err instanceof Error ? err.message : String(err);
        setBootErr((prev) => prev ?? text);
        noteDiag("error", "boot", text);
      });
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
  }, [apply, call, noteDiag]);

  useEffect(() => {
    if (snap) return;
    const timer = window.setTimeout(() => {
      setBootErr((prev) => prev ?? "el core no respondió");
    }, 12000);
    return () => window.clearTimeout(timer);
  }, [snap]);

  useEffect(() => {
    if (!snap) return;
    const live = new Set<number>(snap.held_panes ?? []);
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
    const id = window.setInterval(tick, quotaOpen ? 2500 : 4000);
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
        if (modal?.kind === "copyOnSelect") dismissCopyOnSelectPrompt();
        setModal(null);
        setRenaming(null);
        setRenameWs(false);
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
    return subscribeCopyOnSelectAsk(() => setModal({ kind: "copyOnSelect" }));
  }, []);

  useEffect(() => {
    let timer = 0;
    const off = subscribeCopied(() => {
      setCopiedFlash((n) => n + 1);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setCopiedFlash(0), 1400);
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, []);

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
    try {
      const fill = getComputedStyle(document.documentElement).getPropertyValue("--fill").trim();
      if (fill) localStorage.setItem("lolterm.fill", fill);
      if (!builtin) localStorage.setItem("lolterm.theme", "custom");
    } catch {
      // ignore
    }
  }, [snap]);

  const tab = snap?.tabs[snap.active_tab];
  const remoteHost = tab ? tabRemote(tab) : null;
  const activeWorktree = tab?.panes.find((pane) => pane.id === tab.focused)?.worktree;
  const crumbs = useMemo(() => {
    if (!snap) return ["lolterm"];
    const base = snap.branch ? [snap.name, snap.branch] : [snap.name];
    return activeWorktree ? [...base, `worktree: ${projectName(activeWorktree)}`] : base;
  }, [activeWorktree, snap]);

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

  const splash = !snap;

  return (
    <>
      {snap ? (
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
          <button
            type="button"
            className={modal?.kind === "settings" ? "wm-btn on" : "wm-btn"}
            title="Ajustes"
            onClick={() => setModal({ kind: "settings", tab: "look" })}
          >
            <GearIcon size={12} />
          </button>
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
              <ExplorerPanel
                tree={snap.tree}
                activeRel={tab?.rel}
                onSearch={() => setModal({ kind: "files", query: "" })}
                call={call}
              />
            )}
            {activity === "git" && <GitPanel snap={snap} call={call} />}
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
                    {cli.version && <span className="hint">{cli.version}</span>}
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
                  className={`${on ? "tab-pill on" : "tab-pill"}${remote ? " remote" : ""}${item.kind === "file" && item.rel && dirtyFiles.current[item.rel] ? " dirty" : ""}`}
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
                  <Icon size={12} color={on ? "var(--brand)" : "var(--muted)"} />
                  <span>{item.name}</span>
                  <span
                    className="tab-close"
                    onClick={(e) => {
                      e.stopPropagation();
                      void closeTabAt(index);
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
                {i > 0 && <ChevronRight size={10} color="var(--muted)" />}
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
                onTools={() => setModal({ kind: "settings", tab: "tools" })}
              />
            ) : (
              tab &&
              (tab.kind === "file" && tab.rel ? (
                <Suspense fallback={<div className="doc-view" />}>
                <FileEditor
                  rel={tab.rel}
                  theme={snap.theme}
                  onOpenNvim={() => void call("openInNvim", { rel: tab.rel })}
                  onDirtyChange={markFileDirty}
                  onSaved={() => void call("snapshot")}
                />
                </Suspense>
              ) : tab.kind === "rest" && tab.rel ? (
                <RestClient rel={tab.rel} />
              ) : (
                <SplitView
                  node={tab.layout}
                  panes={tab.panes}
                  focused={tab.focused}
                  zoomed={tab.zoomed}
                  onFocus={(id) => void call("focus", { pane: id })}
                />
              ))
            )}
            {dockEdge && draggingTab != null && draggingTab !== snap.active_tab && (
              <div className={`dock-overlay ${dockEdge}`} />
            )}
          </div>
        </main>
      </div>
      <StatusBar
        snap={snap}
        hud={hud}
        remoteHost={remoteHost ? `${remoteHost}${snap.ssh_tmux_session ? ` · ${snap.ssh_tmux_session}` : ""}` : null}
        quotaOpen={quotaOpen}
        mediaOpen={mediaOpen}
        quotaRef={quotaRef}
        mediaRef={mediaRef}
        banner={banner}
        agentsTick={agentsTick}
        onToggleQuota={() => setQuotaOpen((open) => !open)}
        onToggleMedia={() => setMediaOpen((open) => !open)}
        onMusic={musicAction}
        onSelectTab={(index) => void call("selectTab", { index })}
        onFocusPane={(pane) => void call("focus", { pane })}
        onOpenPort={(port, pane) => {
          void call("focus", { pane });
          void window.lolterm.invoke("openUrl", { url: `http://127.0.0.1:${port}/` });
        }}
        onOpenRoot={() => {
          void window.lolterm.invoke("openRoot");
        }}
        diagCount={diag.filter((row) => row.kind === "error").length}
        onOpenDiag={() => setModal({ kind: "diag" })}
      />
      {tab && tab.kind !== "file" && tab.kind !== "rest" && (
        <TouchBar
          pane={tab.focused}
          onSend={(b64) => {
            void window.lolterm.invoke("write", { pane: tab.focused, b64 });
          }}
        />
      )}
      {copiedFlash > 0 && (
        <div className="copied-toast" key={copiedFlash} role="status">
          Copiado!
        </div>
      )}

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
                <Terminal size={12} color="var(--muted)" />
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
      {modal?.kind === "diag" && (
        <div className="modal" onClick={() => setModal(null)}>
          <DiagnosticsPanel
            entries={diag}
            version={snap.version}
            theme={snap.theme}
            root={snap.root}
            onChange={setDiag}
            onClose={() => setModal(null)}
          />
        </div>
      )}
      {modal?.kind === "settings" && (
        <div className="modal" onClick={() => setModal(null)}>
          <Settings
            snap={snap}
            tab={modal.tab}
            onTab={(tab) => setModal({ kind: "settings", tab })}
            call={call}
            onClose={() => setModal(null)}
            onOpenCommands={() => setModal({ kind: "commands" })}
            onUpdate={() => {
              setModal(null);
              void runBound("app.update");
            }}
            onDiagnostics={() => setModal({ kind: "diag" })}
          />
        </div>
      )}
      {modal?.kind === "copyOnSelect" && (
        <div
          className="modal"
          onClick={() => {
            dismissCopyOnSelectPrompt();
            setModal(null);
          }}
        >
          <div className="card" onClick={(e) => e.stopPropagation()}>
            <h2>copiar al seleccionar</h2>
            <p className="copy-select-copy">
              {copyOnSelectEnabled() === true
                ? "Ya está activo: al marcar texto se copia al portapapeles."
                : copyOnSelectEnabled() === false
                  ? "Está desactivado. Ctrl+Shift+C sigue copiando la selección."
                  : "¿Activar copiar al seleccionar? Al marcar texto en la terminal se copia al portapapeles."}
            </p>
            <button type="button" className="row" onClick={() => chooseCopyOnSelect(true)}>
              sí, copiar al seleccionar
            </button>
            <button type="button" className="row" onClick={() => chooseCopyOnSelect(false)}>
              no, sólo con Ctrl+Shift+C
            </button>
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
      ) : null}
      {splash ? <BootScreen error={bootErr} /> : null}
    </>
  );
}
