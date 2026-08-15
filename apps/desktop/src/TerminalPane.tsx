import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";
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

function ensureTerm(pane: number): Cached {
  const existing = cache.get(pane);
  if (existing) return existing;
  const term = new Terminal({
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    fontSize: 13,
    cursorBlink: true,
    scrollback: 2000,
    theme: xtermTheme(currentTheme),
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
  });
  const entry = { term, fit, off };
  cache.set(pane, entry);
  return entry;
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
    }

    let debounce: number | undefined;
    const sendSize = () => {
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

    return () => {
      window.removeEventListener("resize", schedule);
      ro.disconnect();
      if (debounce != null) window.clearTimeout(debounce);
      if (entry.term.element?.parentElement === node) {
        node.removeChild(entry.term.element);
      }
      pendingDispose.set(
        pane,
        window.setTimeout(() => {
          const cached = cache.get(pane);
          if (!cached) return;
          cached.off();
          cached.term.dispose();
          cache.delete(pane);
          pendingDispose.delete(pane);
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
