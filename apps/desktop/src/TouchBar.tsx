import { b64encode } from "./types";

type Key = { label: string; send: string };

const KEYS: Key[] = [
  { label: "Esc", send: "\x1b" },
  { label: "Tab", send: "\t" },
  { label: "Ctrl-C", send: "\x03" },
  { label: "Ctrl-D", send: "\x04" },
  { label: "Ctrl-Z", send: "\x1a" },
  { label: "↑", send: "\x1b[A" },
  { label: "↓", send: "\x1b[B" },
  { label: "←", send: "\x1b[D" },
  { label: "→", send: "\x1b[C" },
  { label: "Enter", send: "\r" },
];

function encode(text: string) {
  return b64encode(new TextEncoder().encode(text));
}

export function TouchBar({ pane, onSend }: { pane: number | null; onSend: (b64: string) => void }) {
  if (pane == null) return null;
  return (
    <div className="touch-bar" aria-label="teclas de terminal">
      {KEYS.map((key) => (
        <button key={key.label} type="button" onClick={() => onSend(encode(key.send))}>
          {key.label}
        </button>
      ))}
    </div>
  );
}
