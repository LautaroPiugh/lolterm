import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";
import { bindingFor } from "./chords";
import { type ThemeId, xtermTheme } from "./themes";
import { b64decode, b64encode } from "./types";

type Cached = {
  term: Terminal;
  fit: FitAddon;
  off: () => void;
};

const cache = new Map<number, Cached>();
const pendingDispose = new Map<number, number>();
let currentTheme: ThemeId = "sage";

export function applyXtermTheme(id: ThemeId) {
  currentTheme = id;
  const theme = xtermTheme(id);
  for (const entry of cache.values()) {
    entry.term.options.theme = theme;
  }
}

export function disposeTerm(pane: number) {
  const timer = pendingDispose.get(pane);
  if (timer != null) {
    window.clearTimeout(timer);
    pendingDispose.delete(pane);
  }
  const cached = cache.get(pane);
  if (!cached) return;
  cached.off();
  cached.term.dispose();
  cache.delete(pane);
}

async function copySelection(term: Terminal) {
  const text = term.getSelection();
  if (!text) return;
  await window.lolterm.clipboard.write(text);
}

async function pasteInto(term: Terminal) {
  const text = await window.lolterm.clipboard.read();
  if (!text) return;
  term.paste(text);
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
      void window.lolterm.clipboard.read().then((text) => {
        term.input(`\x1b]52;${sel};${utf8ToB64(text)}\x07`, false);
      });
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
  term.attachCustomKeyEventHandler((ev) => {
    if (ev.type !== "keydown") return true;
    if (bindingFor(ev)) return false;
    const chord = ev.ctrlKey || ev.metaKey;
    if (chord && ev.shiftKey && ev.code === "KeyC") {
      void copySelection(term);
      return false;
    }
    if (chord && ev.shiftKey && ev.code === "KeyV") {
      void pasteInto(term);
      return false;
    }
    return true;
  });
  host.addEventListener("auxclick", (ev) => {
    if (ev.button !== 1) return;
    ev.preventDefault();
    void pasteInto(term);
  });
  host.addEventListener("mouseup", () => {
    if (term.hasSelection()) void copySelection(term);
  });
}

function ensureTerm(pane: number): Cached {
  const existing = cache.get(pane);
  if (existing) return existing;
  const term = new Terminal({
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    fontSize: 13,
    cursorBlink: true,
    scrollback: 2000,
    theme: xtermTheme(currentTheme),
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
  const off = window.lolterm.onEvent((msg) => {
    if (msg.event === "data" && msg.params?.pane === pane && msg.params.b64) {
      term.write(b64decode(msg.params.b64));
    }
    if (msg.event === "exit" && msg.params?.pane === pane) {
      disposeTerm(pane);
    }
  });
  const entry = { term, fit, off };
  cache.set(pane, entry);
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
}: {
  pane: number;
  focused: boolean;
  onFocus: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);

  useEffect(() => {
    const node = host.current;
    if (!node) return;
    const timer = pendingDispose.get(pane);
    if (timer != null) {
      window.clearTimeout(timer);
      pendingDispose.delete(pane);
    }
    const entry = ensureTerm(pane);
    termRef.current = entry.term;
    if (entry.term.element) {
      node.appendChild(entry.term.element);
    } else {
      entry.term.open(node);
      wireOsc52(entry.term);
      if (entry.term.element) wireClipboard(entry.term, entry.term.element);
    }

    let debounce: number | undefined;
    const sendSize = () => {
      if (node.clientWidth < 2 || node.clientHeight < 2) return;
      entry.fit.fit();
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
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", schedule);
      ro.disconnect();
      if (debounce != null) window.clearTimeout(debounce);
      if (entry.term.element?.parentElement === node) {
        node.removeChild(entry.term.element);
      }
      pendingDispose.set(
        pane,
        window.setTimeout(() => {
          disposeTerm(pane);
        }, 300),
      );
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
    />
  );
}
