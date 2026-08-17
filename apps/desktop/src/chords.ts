export type Binding = { chord: string; command: string };

const FALLBACK: Binding[] = [
  { chord: "ctrl+b", command: "ui.palette" },
  { chord: "ctrl+p", command: "ui.palette" },
  { chord: "alt+ctrl+h", command: "pane.focusLeft" },
  { chord: "alt+ctrl+l", command: "pane.focusRight" },
  { chord: "alt+ctrl+k", command: "pane.focusUp" },
  { chord: "alt+ctrl+j", command: "pane.focusDown" },
  { chord: "alt+ctrl+v", command: "pane.splitRight" },
  { chord: "alt+ctrl+s", command: "pane.splitDown" },
  { chord: "alt+ctrl+r", command: "pane.restart" },
  { chord: "alt+ctrl+e", command: "ui.tabRename" },
  { chord: "ctrl+tab", command: "tab.next" },
  { chord: "ctrl+shift+tab", command: "tab.prev" },
];

let bindings: Binding[] = FALLBACK;

export function setBindings(next: Binding[] | undefined) {
  bindings = next && next.length > 0 ? next : FALLBACK;
}

export function eventChord(e: KeyboardEvent): string {
  const mods: string[] = [];
  if (e.altKey) mods.push("alt");
  if (e.ctrlKey) mods.push("ctrl");
  if (e.metaKey) mods.push("meta");
  if (e.shiftKey) mods.push("shift");
  let key = e.key.length === 1 ? e.key.toLowerCase() : e.key.toLowerCase();
  if (e.code.startsWith("Key") && e.code.length === 4) {
    key = e.code.slice(3).toLowerCase();
  }
  return mods.length ? `${mods.join("+")}+${key}` : key;
}

export function bindingFor(e: KeyboardEvent): Binding | undefined {
  const chord = eventChord(e);
  return bindings.find((item) => item.chord === chord);
}

export function commandForChord(chord: string): string | undefined {
  return bindings.find((item) => item.chord === chord)?.command;
}

export function isChromeField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.classList.contains("xterm-helper-textarea")) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
}
