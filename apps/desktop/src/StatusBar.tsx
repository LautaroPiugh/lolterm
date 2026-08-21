import { useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { AlertTriangle, GitBranch, Network, Sparkles, Terminal } from "./icons";
import { MediaDock, QuotaButton } from "./Hud";
import { displayVersion, eraLabel } from "./version";
import type { Hud, Snapshot } from "./types";

type Pop = "host" | "ports" | "procs" | "agents" | null;

type Props = {
  snap: Snapshot;
  hud: Hud | null;
  remoteHost: string | null;
  quotaOpen: boolean;
  mediaOpen: boolean;
  quotaRef: RefObject<HTMLDivElement | null>;
  mediaRef: RefObject<HTMLDivElement | null>;
  banner: string | null;
  diagCount: number;
  onOpenDiag: () => void;
  agentsTick: number;
  onToggleQuota: () => void;
  onToggleMedia: () => void;
  onMusic: (action: string, volume?: number) => void;
  onSelectTab: (index: number) => void;
  onFocusPane: (pane: number) => void;
  onOpenPort: (port: number, pane: number) => void;
  onOpenRoot: () => void;
};

export function StatusBar({
  snap,
  hud,
  remoteHost,
  quotaOpen,
  mediaOpen,
  quotaRef,
  mediaRef,
  banner,
  diagCount,
  onOpenDiag,
  agentsTick,
  onToggleQuota,
  onToggleMedia,
  onMusic,
  onSelectTab,
  onFocusPane,
  onOpenPort,
  onOpenRoot,
}: Props) {
  const [pop, setPop] = useState<Pop>(null);
  const wrapRef = useRef<HTMLElement | null>(null);
  const agents = snap.agents ?? [];
  const waiting = agents.filter((agent) => agent.attention);
  const active = agents.filter((agent) => !agent.attention);
  const ports = hud?.extra?.ports ?? [];
  const procs = hud?.extra?.processes ?? [];

  useEffect(() => {
    if (agentsTick > 0 && agents.length > 0) setPop("agents");
  }, [agentsTick, agents.length]);

  useEffect(() => {
    if (!pop) return;
    const onDown = (event: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(event.target as Node)) {
        setPop(null);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [pop]);

  const toggle = (next: Pop) => setPop((cur) => (cur === next ? null : next));

  return (
    <footer className="status" ref={wrapRef}>
      <span className="status-item status-version" title={eraLabel(snap.version)}>
        {displayVersion(snap.version)}
      </span>
      <span className="status-sep">·</span>
      <span className="status-item">
        <GitBranch size={11} />
        {snap.git?.branch ?? "—"}
      </span>
      <span className="status-sep">·</span>
      <button type="button" className="status-chip" title={snap.root} onClick={onOpenRoot}>
        <span className="status-path">{snap.root}</span>
      </button>

      {(hud?.host?.load || hud?.extra?.disk != null) && (
        <Chip
          open={pop === "host"}
          label={[
            hud?.host?.load,
            hud?.host?.mem != null ? `${hud.host.mem}% ram` : null,
            hud?.extra?.disk != null ? `${hud.extra.disk}% disk` : null,
            hud?.extra?.battery != null ? `bat ${hud.extra.battery}%` : null,
          ]
            .filter(Boolean)
            .join(" · ")}
          title="host"
          onToggle={() => toggle("host")}
        >
          <p className="status-pop-kicker">workspace</p>
          <button type="button" className="status-hit" onClick={onOpenRoot}>
            abrir carpeta · xdg-open
          </button>
          {hud?.extra?.disk != null && <p className="status-pop-note">disco del root: {hud.extra.disk}% usado</p>}
          {hud?.extra?.battery != null && <p className="status-pop-note">batería: {hud.extra.battery}%</p>}
        </Chip>
      )}

      {remoteHost && (
        <>
          <span className="status-sep">·</span>
          <span className="status-item status-remote">{remoteHost}</span>
        </>
      )}

      {agents.length > 0 && (
        <Chip
          open={pop === "agents"}
          hot={waiting.length > 0}
          label={
            waiting.length > 0 ? `${waiting.length} waiting · ${agents.length}` : `${agents.length} agents`
          }
          title="Attention"
          icon={<Sparkles size={11} />}
          onToggle={() => toggle("agents")}
        >
          {waiting.length > 0 && (
            <>
              <p className="status-pop-kicker">waiting</p>
              {waiting.map((agent) => (
                <button
                  key={`${agent.tab}-${agent.program}-w`}
                  type="button"
                  className="status-hit"
                  onClick={() => {
                    onSelectTab(agent.tab);
                    setPop(null);
                  }}
                >
                  {agent.program}
                  <span className="hint">{agent.tab_name}</span>
                </button>
              ))}
            </>
          )}
          {active.length > 0 && (
            <>
              <p className="status-pop-kicker">active</p>
              {active.map((agent) => (
                <button
                  key={`${agent.tab}-${agent.program}-a`}
                  type="button"
                  className="status-hit"
                  onClick={() => {
                    onSelectTab(agent.tab);
                    setPop(null);
                  }}
                >
                  {agent.program}
                  <span className="hint">{agent.tab_name}</span>
                </button>
              ))}
            </>
          )}
        </Chip>
      )}

      {ports.length > 0 && (
        <Chip
          open={pop === "ports"}
          label={`${ports.length} ports`}
          title="puertos listen de los PTYs"
          icon={<Network size={11} />}
          onToggle={() => toggle("ports")}
        >
          <p className="status-pop-kicker">listen · clic abre localhost</p>
          {ports.map((row) => (
            <button
              key={`${row.pid}-${row.port}`}
              type="button"
              className="status-hit"
              onClick={() => {
                onOpenPort(row.port, row.pane);
                setPop(null);
              }}
            >
              {row.program}:{row.port}
              <span className="hint">pane {row.pane}</span>
            </button>
          ))}
        </Chip>
      )}

      {procs.length > 0 && (
        <Chip
          open={pop === "procs"}
          label={`${procs.length} procs`}
          title="procesos de los PTYs"
          icon={<Terminal size={11} />}
          onToggle={() => toggle("procs")}
        >
          <p className="status-pop-kicker">hijos del mux</p>
          {procs.map((row) => (
            <button
              key={row.pid}
              type="button"
              className="status-hit"
              onClick={() => {
                onFocusPane(row.pane);
                setPop(null);
              }}
            >
              {row.program}
              <span className="hint">{row.pid}</span>
            </button>
          ))}
        </Chip>
      )}

      {(snap.status_ext ?? []).map((item) => (
        <span key={item.id} className="status-item" title={item.id}>
          {item.text}
        </span>
      ))}

      <div className="status-end">
        <QuotaButton
          open={quotaOpen}
          agents={hud?.quota ?? []}
          onToggle={onToggleQuota}
          wrapRef={quotaRef}
        />
        <div className="status-media-wrap" ref={mediaRef}>
          <button
            type="button"
            className="status-media"
            title={
              hud?.music
                ? `${hud.music.source ?? "media"} · ${hud.music.title}`
                : hud?.playerctl
                  ? "media (playerctl)"
                  : "instalá playerctl"
            }
            onClick={onToggleMedia}
          >
            {hud?.music ? `${hud.music.playing ? "▶ " : ""}${hud.music.title}` : "media"}
          </button>
          {mediaOpen && <MediaDock hud={hud} open={mediaOpen} onClose={onToggleMedia} onAction={onMusic} />}
        </div>
      </div>
      {diagCount > 0 && (
        <>
          <span className="status-sep">·</span>
          <button type="button" className="status-chip status-diag" title="Diagnóstico" onClick={onOpenDiag}>
            <AlertTriangle size={11} />
            {diagCount} {diagCount === 1 ? "error" : "errores"}
          </button>
        </>
      )}
      {banner && <span className="notice">{banner}</span>}
    </footer>
  );
}

function Chip({
  open,
  hot,
  label,
  title,
  icon,
  onToggle,
  children,
}: {
  open: boolean;
  hot?: boolean;
  label: string;
  title: string;
  icon?: ReactNode;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <div className="status-chip-wrap">
      <span className="status-sep">·</span>
      <button
        type="button"
        className={open ? "status-chip on" : "status-chip"}
        data-hot={hot ? "1" : "0"}
        title={title}
        onClick={onToggle}
      >
        {icon}
        {label}
      </button>
      {open && <div className="status-pop">{children}</div>}
    </div>
  );
}
