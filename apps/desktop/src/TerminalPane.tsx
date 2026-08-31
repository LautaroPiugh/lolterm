import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";
import { X } from "./icons";
import { bindingFor } from "./chords";
import { maybeCopySelection, writeClipboard } from "./copyOnSelect";
import { parseTheme, type ThemeId, xtermTheme } from "./themes";
import { b64decode, b64encode } from "./types";

type Cached = {
  term: Terminal;
  fit: FitAddon;
};

const cache = new Map<number, Cached>();
const backlog = new Map<number, string[]>();
const BACKLOG_CHARS = 1_500_000;
let currentTheme: ThemeId = "claro";
let lastXterm = xtermTheme("claro");
let lastTitles = new Map<number, string>();
let reportPaneTitle: ((pane: number, title: string) => void) | undefined;

function ingestData(pane: number, b64: string) {
  const cached = cache.get(pane);
  if (cached) {
    cached.term.write(b64decode(b64));
    return;
  }
  const q = backlog.get(pane) ?? [];
  q.push(b64);
  let chars = 0;
  for (const chunk of q) chars += chunk.length;
  while (q.length > 1 && chars > BACKLOG_CHARS) {
    chars -= q.shift()?.length ?? 0;
  }
  backlog.set(pane, q);
}

if (typeof window !== "undefined" && window.lolterm?.onEvent) {
  window.lolterm.onEvent((msg) => {
    if (msg.event === "exit" && msg.params?.pane != null) {
      disposeTerm(msg.params.pane);
      return;
    }
    if (msg.event === "data" && msg.params?.pane != null && msg.params.b64) {
      ingestData(msg.params.pane, msg.params.b64);
    }
  });
}

export function setPaneTitleHandler(handler?: (pane: number, title: string) => void) {
  reportPaneTitle = handler;
}

export function applyXtermTheme(id: string, vars?: Record<string, string>) {
  currentTheme = parseTheme(id);
  lastXterm = vars
    ? {
        background: vars.pane ?? vars.fill ?? "#f4f7f4",
        foreground: vars.text ?? "#28302a",
        cursor: vars.brand ?? "#488c58",
        cursorAccent: vars.pane ?? vars.fill ?? "#f4f7f4",
        selectionBackground: vars.focus ?? vars.brand ?? "#a8d4b0",
      }
    : xtermTheme(currentTheme);
  for (const entry of cache.values()) {
    entry.term.options.theme = lastXterm;
  }
}

export function disposeTerm(pane: number) {
  const cached = cache.get(pane);
  backlog.delete(pane);
  if (!cached) return;
  cached.term.dispose();
  cache.delete(pane);
  lastTitles.delete(pane);
}

export function retainPanes(live: Iterable<number>) {
  const keep = new Set(live);
  for (const id of [...cache.keys()]) {
    if (!keep.has(id)) disposeTerm(id);
  }
}

async function copySelection(term: Terminal) {
  await writeClipboard(term.getSelection());
}

function utf8ToB64(text: string): string {
  return btoa(unescape(encodeURIComponent(text)));
}

function b64ToUtf8(text: string): string {
  return decodeURIComponent(escape(atob(text)));
}

function wireOsc52(term: Terminal) {
  term.parser.registerOscHandler(52, (data) => {
    const sep = data.indexOf(";");
    const sel = sep >= 0 ? data.slice(0, sep) : data;
    const payload = sep >= 0 ? data.slice(sep + 1) : "";
    if (payload === "?") {
      // OSC 52 clipboard reads can leak the local clipboard to a remote PTY
      // (for example through ssh). Keep copy-to-clipboard support, but never
      // answer readback queries.
      return true;
    }
    try {
      void window.lolterm.clipboard.write(payload ? b64ToUtf8(payload) : "");
    } catch {
      void window.lolterm.clipboard.write("");
    }
    return true;
  });
}

function wireClipboard(term: Terminal, host: HTMLElement) {
  let selTimer = 0;
  let lastCopied = "";
  const scheduleCopyOnSelect = () => {
    window.clearTimeout(selTimer);
    selTimer = window.setTimeout(() => {
      const text = term.getSelection();
      if (!text) {
        lastCopied = "";
        return;
      }
      if (text === lastCopied) return;
      lastCopied = text;
      maybeCopySelection(text);
    }, 80);
  };

  term.attachCustomKeyEventHandler((ev) => {
    if (ev.type !== "keydown") return true;
    if (bindingFor(ev)) return false;
    const chord = ev.ctrlKey || ev.metaKey;
    if (chord && ev.shiftKey && ev.code === "KeyC") {
      void copySelection(term);
      return false;
    }
    if (chord && ev.shiftKey && ev.code === "KeyV") {
      // El evento `paste` del DOM pega una sola vez. Si también llamamos
      // term.paste() acá, el texto entra duplicado al PTY.
      return false;
    }
    if (chord && ev.shiftKey && ev.code === "KeyA") {
      term.selectAll();
      maybeCopySelection(term.getSelection());
      return false;
    }
    return true;
  });

  host.addEventListener(
    "paste",
    (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      const text = ev.clipboardData?.getData("text/plain") ?? "";
      if (text) {
        term.paste(text);
        return;
      }
      void window.lolterm.clipboard.read().then((clip) => {
        if (clip) term.paste(clip);
      });
    },
    true,
  );

  host.addEventListener("copy", (ev) => {
    const text = term.getSelection();
    if (!text) return;
    ev.preventDefault();
    ev.clipboardData?.setData("text/plain", text);
    void writeClipboard(text);
  });

  term.onSelectionChange(scheduleCopyOnSelect);
}

function isAltScreen(term: Terminal): boolean {
  return term.buffer.active.type === "alternate";
}

function paintTermScroll(term: Terminal, track: HTMLElement) {
  const thumb = track.querySelector(".term-scroll-thumb") as HTMLElement | null;
  if (!thumb) return;
  const buf = term.buffer.active;
  const total = buf.length;
  const rows = term.rows;
  if (isAltScreen(term) || total <= rows || track.clientHeight < 2) {
    track.hidden = true;
    return;
  }
  track.hidden = false;
  const trackH = track.clientHeight;
  const thumbH = Math.max((rows / total) * trackH, 16);
  const max = total - rows;
  const top = (buf.viewportY / max) * (trackH - thumbH);
  thumb.style.height = `${thumbH}px`;
  thumb.style.transform = `translateY(${Math.max(0, top)}px)`;
}

function reportsMouse(term: Terminal): boolean {
  return term.modes.mouseTrackingMode !== "none";
}

function wireNvimWheel(term: Terminal, host: HTMLElement, pane: number): () => void {
  const onWheel = (ev: WheelEvent) => {
    // Sin pantalla alternada el scrollback de xterm maneja la rueda.
    // Si el programa reporta mouse (CLIs de IA con UI), la rueda es del programa:
    // xterm la traduce a botones SGR y va al PTY; no se intercepta.
    if (!isAltScreen(term) || reportsMouse(term)) return;
    ev.preventDefault();
    ev.stopPropagation();
    const raw =
      ev.deltaMode === WheelEvent.DOM_DELTA_LINE ? Math.abs(ev.deltaY) : Math.abs(ev.deltaY) / 40;
    const steps = Math.min(15, Math.max(1, Math.round(raw) || 1));
    const bytes = new TextEncoder().encode((ev.deltaY < 0 ? "\x19" : "\x05").repeat(steps));
    void window.lolterm.invoke("write", { pane, b64: b64encode(bytes) });
  };
  host.addEventListener("wheel", onWheel, { passive: false, capture: true });
  term.attachCustomWheelEventHandler(() => !isAltScreen(term) || reportsMouse(term));
  return () => {
    host.removeEventListener("wheel", onWheel, { capture: true } as AddEventListenerOptions);
  };
}
function wireTermScroll(term: Terminal, track: HTMLElement): () => void {
  const thumb = track.querySelector(".term-scroll-thumb") as HTMLElement | null;
  let raf = 0;
  const schedule = () => {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      paintTermScroll(term, track);
    });
  };
  const scrollToY = (clientY: number) => {
    if (!thumb) return;
    if (isAltScreen(term)) return;
    const max = term.buffer.active.length - term.rows;
    if (max <= 0) return;
    const rect = track.getBoundingClientRect();
    const usable = Math.max(rect.height - thumb.offsetHeight, 1);
    const ratio = Math.min(1, Math.max(0, (clientY - rect.top - thumb.offsetHeight / 2) / usable));
    term.scrollToLine(Math.round(ratio * max));
  };
  const onDown = (ev: MouseEvent) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    scrollToY(ev.clientY);
    const onMove = (e: MouseEvent) => scrollToY(e.clientY);
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };
  track.addEventListener("mousedown", onDown);
  const renderDisp = term.onRender(schedule);
  const scrollDisp = term.onScroll(schedule);
  schedule();
  return () => {
    if (raf) cancelAnimationFrame(raf);
    renderDisp.dispose();
    scrollDisp.dispose();
    track.removeEventListener("mousedown", onDown);
  };
}

function ensureTerm(pane: number): Cached {
  const existing = cache.get(pane);
  if (existing) return existing;
  const term = new Terminal({
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    fontSize: 13,
    cursorBlink: true,
    scrollback: 2000,
    theme: lastXterm,
    allowProposedApi: true,
    rightClickSelectsWord: true,
    macOptionIsMeta: true,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  term.onData((data) => {
    const bytes = new TextEncoder().encode(data);
    void window.lolterm.invoke("write", { pane, b64: b64encode(bytes) });
  });
  term.onTitleChange((title) => {
    const next = title.trim();
    if (!next || lastTitles.get(pane) === next) return;
    lastTitles.set(pane, next);
    reportPaneTitle?.(pane, next);
  });
  const entry = { term, fit };
  cache.set(pane, entry);
  const queued = backlog.get(pane);
  backlog.delete(pane);
  if (queued) {
    for (const chunk of queued) term.write(b64decode(chunk));
  }
  return entry;
}

export function refitAllTerminals() {
  requestAnimationFrame(() => {
    for (const [pane, entry] of cache) {
      entry.fit.fit();
      void window.lolterm.invoke("resize", {
        pane,
        cols: entry.term.cols,
        rows: entry.term.rows,
      });
    }
  });
}

export function TerminalPane({
  pane,
  focused,
  onFocus,
  closable = false,
  onClose,
}: {
  pane: number;
  focused: boolean;
  onFocus: () => void;
  closable?: boolean;
  onClose?: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const track = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);

  useEffect(() => {
    const node = host.current;
    const scroll = track.current;
    if (!node || !scroll) return;
    const entry = ensureTerm(pane);
    termRef.current = entry.term;
    if (entry.term.element) {
      node.insertBefore(entry.term.element, scroll);
    } else {
      entry.term.open(node);
      wireOsc52(entry.term);
      if (entry.term.element) {
        wireClipboard(entry.term, entry.term.element);
        node.insertBefore(entry.term.element, scroll);
      }
    }
    const unwireScroll = wireTermScroll(entry.term, scroll);
    const unwireWheel = wireNvimWheel(entry.term, node, pane);

    let debounce: number | undefined;
    const sendSize = () => {
      if (node.clientWidth < 2 || node.clientHeight < 2) return;
      entry.fit.fit();
      paintTermScroll(entry.term, scroll);
      void window.lolterm.invoke("resize", {
        pane,
        cols: entry.term.cols,
        rows: entry.term.rows,
      });
    };
    const schedule = () => {
      if (debounce != null) window.clearTimeout(debounce);
      debounce = window.setTimeout(sendSize, 50);
    };
    const ro = new ResizeObserver(schedule);
    ro.observe(node);
    window.addEventListener("resize", schedule);
    queueMicrotask(sendSize);
    const raf = requestAnimationFrame(sendSize);

    return () => {
      unwireScroll();
      unwireWheel();
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", schedule);
      ro.disconnect();
      if (debounce != null) window.clearTimeout(debounce);
      if (entry.term.element?.parentElement === node) {
        node.removeChild(entry.term.element);
      }
    };
  }, [pane]);

  useEffect(() => {
    if (focused) termRef.current?.focus();
  }, [focused]);

  return (
    <div
      className={`term ${focused ? "term-focus" : ""}`}
      onMouseDown={() => {
        onFocus();
        termRef.current?.focus();
      }}
      ref={host}
    >
      {closable && onClose ? (
        <button
          type="button"
          className="term-close"
          title="Cerrar terminal"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
        >
          <X size={11} />
        </button>
      ) : null}
      <div className="term-scroll" hidden ref={track}>
        <div className="term-scroll-thumb" />
      </div>
    </div>
  );
}
