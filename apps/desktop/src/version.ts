const ERAS = [
  "Foundation",
  "Multiplexer",
  "Workspaces",
  "Remote",
  "CLI",
  "Context",
  "AI host",
  "Extensibility",
  "Stabilization",
] as const;

export function displayVersion(raw: string | undefined | null): string {
  const value = (raw ?? "").trim();
  if (!value) return "v0.0.0";
  return value.startsWith("v") ? value : `v${value}`;
}

export function eraLabel(raw: string | undefined | null): string {
  const value = (raw ?? "").trim().replace(/^v/, "");
  const minor = Number(value.split(".")[1] ?? 0);
  if (value.startsWith("1.")) return "Personal Environment";
  return ERAS[minor - 1] ?? "LoLTerm";
}
