const KEY = "lolterm.diag";
const MAX = 80;

export type DiagKind = "error" | "warn" | "info";

export type DiagEntry = {
  ts: number;
  kind: DiagKind;
  source: string;
  message: string;
};

function load(): DiagEntry[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    if (!Array.isArray(raw)) return [];
    return raw.filter(
      (row) =>
        row &&
        typeof row.ts === "number" &&
        typeof row.source === "string" &&
        typeof row.message === "string",
    );
  } catch {
    return [];
  }
}

function save(rows: DiagEntry[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(rows.slice(0, MAX)));
  } catch {
    // quota; el visor sigue en memoria
  }
}

function redactPath(raw?: string): string {
  const text = String(raw ?? "").trim();
  if (!text) return "?";
  const normalized = text.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  const name = parts.at(-1);
  return name ? `…/${name}` : "…";
}


export function readDiag(): DiagEntry[] {
  return load();
}

export function pushDiag(kind: DiagKind, source: string, message: string): DiagEntry[] {
  const text = message.trim().slice(0, 500);
  if (!text) return load();
  const rows = load();
  const last = rows[0];
  const windowMs = kind === "info" ? 60_000 : 2000;
  if (last && last.source === source && last.message === text && Date.now() - last.ts < windowMs) {
    return rows;
  }
  const next = [{ ts: Date.now(), kind, source, message: text }, ...rows].slice(0, MAX);
  save(next);
  return next;
}

export function clearDiag(): DiagEntry[] {
  save([]);
  return [];
}

export function formatReport(
  entries: DiagEntry[],
  meta: { version?: string; theme?: string; root?: string },
): string {
  const header = [
    `LoLTerm ${meta.version ?? "?"}`,
    `tema: ${meta.theme ?? "?"}`,
    `root: ${redactPath(meta.root)}`,
    `ua: ${navigator.userAgent}`,
    "",
    "errores recientes (sin secretos ni salida de PTY):",
  ];
  const body = entries.slice(0, 20).map((row) => {
    const when = new Date(row.ts).toISOString();
    return `- ${when} [${row.kind}] ${row.source}: ${row.message}`;
  });
  return [...header, ...(body.length ? body : ["- (vacío)"]), ""].join("\n");
}
