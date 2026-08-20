const KEY = "lolterm.copyOnSelect";

const askers = new Set<() => void>();
const copied = new Set<() => void>();
let promptOpen = false;
let pending = "";

/** `null` = todavía no eligió (hay que preguntar). */
export function copyOnSelectEnabled(): boolean | null {
  try {
    const value = localStorage.getItem(KEY);
    if (value === "on") return true;
    if (value === "off") return false;
  } catch {
    return null;
  }
  return null;
}

export function setCopyOnSelect(on: boolean) {
  localStorage.setItem(KEY, on ? "on" : "off");
  promptOpen = false;
}

export function subscribeCopyOnSelectAsk(cb: () => void) {
  askers.add(cb);
  return () => {
    askers.delete(cb);
  };
}

export function askCopyOnSelect() {
  if (promptOpen) return;
  promptOpen = true;
  for (const cb of askers) cb();
}

export function dismissCopyOnSelectPrompt() {
  promptOpen = false;
}

export function stashPendingCopy(text: string) {
  pending = text;
}

export function takePendingCopy(): string {
  const text = pending;
  pending = "";
  return text;
}

export function subscribeCopied(cb: () => void) {
  copied.add(cb);
  return () => {
    copied.delete(cb);
  };
}

export async function writeClipboard(text: string) {
  if (!text) return;
  await window.lolterm.clipboard.write(text);
  for (const cb of copied) cb();
}

export function maybeCopySelection(text: string) {
  if (!text) return;
  const enabled = copyOnSelectEnabled();
  if (enabled === true) {
    void writeClipboard(text);
    return;
  }
  if (enabled === false) return;
  stashPendingCopy(text);
  askCopyOnSelect();
}
