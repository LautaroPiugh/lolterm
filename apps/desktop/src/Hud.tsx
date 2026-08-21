import type { RefObject } from "react";
import { Pause, Play, SkipBack, SkipForward, Volume2 } from "./icons";
import type { Hud, QuotaAgent } from "./types";

type QuotaProps = {
  open: boolean;
  agents: QuotaAgent[];
  onToggle: () => void;
  wrapRef: RefObject<HTMLDivElement | null>;
};

export function QuotaButton({ open, agents, onToggle, wrapRef }: QuotaProps) {
  const visible = agents.filter(
    (agent) => agent.supported || agent.pending || agent.available || agent.running || agent.note,
  );
  const leftovers = visible.flatMap((agent) => agent.bars).map((bar) => Math.max(0, 100 - bar.percent));
  const lowestLeft = leftovers.length ? Math.min(...leftovers) : null;
  const worstUsed = lowestLeft == null ? 0 : 100 - lowestLeft;
  return (
    <div className="quota-wrap" ref={wrapRef}>
      <button
        type="button"
        className={open ? "quota-btn on" : "quota-btn"}
        title="Cuota de agentes"
        onClick={onToggle}
      >
        Quota
        {lowestLeft != null && <span className="quota-pct">{lowestLeft}%</span>}
        {worstUsed > 0 && <span className="quota-dot" data-hot={worstUsed >= 80 ? "1" : "0"} />}
      </button>
      {open && (
        <div className="quota-menu">
          <div className="quota-head">
            <strong>Agent quota</strong>
            <span>CLIs instaladas · barras si la CLI publica cuota</span>
          </div>
          {visible.length === 0 && <p className="quota-note">Ningún agente con cuota detectable.</p>}
          {visible.map((agent) => (
            <QuotaRow key={agent.id} agent={agent} />
          ))}
        </div>
      )}
    </div>
  );
}

function QuotaRow({ agent }: { agent: QuotaAgent }) {
  return (
    <div className="quota-agent">
      <div className="quota-agent-title">
        <span>{agent.label}</span>
        <span className="quota-flags">
          {agent.pending && !agent.bars.length
            ? "…"
            : agent.running
              ? "en un pane"
              : "instalado"}
        </span>
      </div>
      {agent.bars.length === 0 && (
        <p className="quota-note">{agent.note ?? (agent.pending ? "consultando…" : "sin ventana de cuota")}</p>
      )}
      {agent.bars.map((bar) => {
        const used = bar.percent;
        const left = Math.max(0, 100 - used);
        const hot = used > 85;
        const warm = used > 60;
        return (
          <div key={bar.key} className="quota-bar-row">
            <div className="quota-bar-meta">
              <span>{bar.label}</span>
              <span>
                {left}% left{bar.reset ? ` · ${bar.reset}` : ""}
              </span>
            </div>
            <div className="quota-track" aria-hidden>
              <div
                className="quota-fill"
                data-hot={hot ? "1" : "0"}
                data-warm={warm && !hot ? "1" : "0"}
                style={{ width: `${used}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

type MediaProps = {
  hud: Hud | null;
  open: boolean;
  onClose: () => void;
  onAction: (action: string, volume?: number) => void;
};

export function MediaDock({ hud, open, onClose, onAction }: MediaProps) {
  const music = hud?.music;
  if (!open && !music) return null;
  const letter = (music?.title || "?").trim().charAt(0).toUpperCase();
  return (
    <aside className="media-dock" aria-label="Reproductor">
      <button type="button" className="media-dismiss" title="cerrar" onClick={onClose}>
        ×
      </button>
      <div className="media-main">
        <div className="media-art" aria-hidden>
          {music?.art ? (
            <img src={music.art} alt="" draggable={false} />
          ) : (
            <span>{letter}</span>
          )}
        </div>
        <div className="media-body">
          <div className="media-title">{music?.title ?? "Sin reproductor"}</div>
          <div className="media-artist">{music?.artist || music?.source || "media"}</div>
          <div className="media-controls">
            <button type="button" title="anterior" onClick={() => onAction("prev")} disabled={!hud?.playerctl}>
              <SkipBack size={15} />
            </button>
            <button
              type="button"
              className="media-play"
              title="play/pause"
              onClick={() => onAction("playPause")}
              disabled={!hud?.playerctl}
            >
              {music?.playing ? <Pause size={15} /> : <Play size={15} />}
            </button>
            <button type="button" title="siguiente" onClick={() => onAction("next")} disabled={!hud?.playerctl}>
              <SkipForward size={15} />
            </button>
          </div>
        </div>
      </div>
      <label className="media-vol">
        <Volume2 size={13} />
        <input
          type="range"
          min={0}
          max={100}
          value={Math.round((hud?.volume ?? 0) * 100)}
          disabled={hud?.sink === false}
          onInput={(event) => onAction("volume", Number(event.currentTarget.value) / 100)}
        />
      </label>
    </aside>
  );
}
