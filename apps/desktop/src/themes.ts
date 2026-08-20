import type { ITheme } from "@xterm/xterm";

/** Escala Orquester: 50 texto → 950 fondo (se invierte en claro). */
type Scale = {
  "50": string;
  "100": string;
  "200": string;
  "300": string;
  "400": string;
  "500": string;
  "600": string;
  "700": string;
  "800": string;
  "900": string;
  "950": string;
};

type Palette = { mode: "light" | "dark"; n: Scale };

const STEPS = ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"] as const;

const scale = (...values: string[]): Scale => ({
  "50": values[0],
  "100": values[1],
  "200": values[2],
  "300": values[3],
  "400": values[4],
  "500": values[5],
  "600": values[6],
  "700": values[7],
  "800": values[8],
  "900": values[9],
  "950": values[10],
});

/** Paletas de https://github.com/sammwyy/orquester (design-tokens). */
export const PALETTES: Record<string, Palette> = {
  dusk: {
    mode: "dark",
    n: scale("245 247 245", "232 235 232", "213 216 213", "191 195 191", "161 167 162", "132 140 134", "88 105 92", "64 78 68", "44 54 47", "27 34 29", "15 19 16"),
  },
  sage: {
    mode: "light",
    n: scale("15 17 15", "28 30 29", "46 48 47", "68 71 69", "96 101 98", "121 127 123", "147 153 149", "190 218 200", "218 240 224", "240 250 243", "250 254 251"),
  },
  slate: {
    mode: "dark",
    n: scale("246 247 248", "231 233 234", "211 214 215", "190 194 195", "158 164 166", "128 136 139", "69 97 116", "48 72 89", "31 49 62", "19 31 42", "11 19 28"),
  },
  "slate-light": {
    mode: "light",
    n: scale("14 15 16", "27 28 29", "45 46 47", "66 68 69", "94 98 99", "119 124 125", "145 151 152", "200 216 224", "222 235 240", "242 248 250", "250 253 254"),
  },
  warm: {
    mode: "dark",
    n: scale("250 250 249", "245 245 244", "231 229 228", "214 211 209", "168 162 158", "120 113 108", "87 83 78", "68 64 60", "41 37 36", "28 25 23", "14 12 11"),
  },
  "warm-light": {
    mode: "light",
    n: scale("12 10 9", "28 25 23", "41 37 36", "68 64 60", "106 100 94", "128 121 114", "152 145 138", "214 209 203", "231 227 221", "244 241 237", "253 252 250"),
  },
  rose: {
    mode: "dark",
    n: scale("248 247 247", "235 233 234", "216 214 215", "194 191 192", "165 160 162", "138 132 134", "108 76 84", "79 58 64", "55 43 46", "35 29 30", "20 17 18"),
  },
  "rose-light": {
    mode: "light",
    n: scale("18 17 18", "31 30 31", "50 49 50", "72 70 71", "100 96 98", "125 120 122", "151 146 148", "226 184 190", "241 218 222", "253 242 243", "255 250 250"),
  },
  dune: {
    mode: "dark",
    n: scale("247 247 243", "235 234 229", "216 214 205", "195 191 177", "165 159 137", "136 130 105", "101 91 56", "75 68 43", "53 48 32", "34 31 21", "20 18 12"),
  },
  "dune-light": {
    mode: "light",
    n: scale("19 18 14", "32 31 27", "51 50 44", "74 72 62", "101 98 83", "126 122 101", "153 148 123", "231 215 161", "244 235 202", "252 248 232", "255 253 245"),
  },
  amethyst: {
    mode: "dark",
    n: scale("247 246 248", "235 233 237", "216 213 219", "192 187 196", "164 155 173", "134 124 145", "103 77 119", "75 57 88", "52 40 61", "34 27 41", "20 16 25"),
  },
  "amethyst-light": {
    mode: "light",
    n: scale("18 17 19", "31 30 32", "50 48 52", "73 70 76", "101 96 106", "126 119 132", "152 145 159", "216 201 224", "235 228 241", "247 244 249", "252 250 253"),
  },
  "mono-dark": {
    mode: "dark",
    n: scale("250 250 250", "245 245 245", "229 229 229", "212 212 212", "163 163 163", "115 115 115", "82 82 82", "64 64 64", "38 38 38", "23 23 23", "10 10 10"),
  },
  mono: {
    mode: "light",
    n: scale("9 9 11", "24 24 27", "39 39 42", "63 63 70", "100 100 106", "124 124 132", "148 148 156", "208 208 212", "226 226 230", "240 240 243", "252 252 253"),
  },
};

export type ThemeId = keyof typeof PALETTES | string;

export const THEMES: { id: string; label: string; hint: string }[] = [
  { id: "dusk", label: "Matcha", hint: "oscuro" },
  { id: "sage", label: "Matcha", hint: "claro" },
  { id: "slate", label: "Slate", hint: "oscuro" },
  { id: "slate-light", label: "Slate", hint: "claro" },
  { id: "warm", label: "Warm", hint: "oscuro" },
  { id: "warm-light", label: "Warm", hint: "claro" },
  { id: "rose", label: "Rose", hint: "oscuro" },
  { id: "rose-light", label: "Rose", hint: "claro" },
  { id: "dune", label: "Dune", hint: "oscuro" },
  { id: "dune-light", label: "Dune", hint: "claro" },
  { id: "amethyst", label: "Amethyst", hint: "oscuro" },
  { id: "amethyst-light", label: "Amethyst", hint: "claro" },
  { id: "mono-dark", label: "Mono", hint: "oscuro" },
  { id: "mono", label: "Mono", hint: "claro" },
];

export function isBuiltinTheme(id: string): boolean {
  return Object.hasOwn(PALETTES, id);
}

export function parseTheme(value: string | null | undefined): string {
  if (value && isBuiltinTheme(value)) return value;
  return "sage";
}

function rgbToHex(triplet: string): string {
  const [r, g, b] = triplet.split(/\s+/).map(Number);
  return `#${[r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("")}`;
}

const ANSI_DARK: ITheme = {
  red: "#f87171",
  green: "#4ade80",
  yellow: "#fbbf24",
  blue: "#60a5fa",
  magenta: "#c084fc",
  cyan: "#22d3ee",
  white: "#d4d4d8",
  brightBlack: "#52525b",
  brightRed: "#fca5a5",
  brightGreen: "#86efac",
  brightYellow: "#fde68a",
  brightBlue: "#93c5fd",
  brightMagenta: "#d8b4fe",
  brightCyan: "#67e8f9",
  brightWhite: "#fafafa",
};

const ANSI_LIGHT: ITheme = {
  red: "#b91c1c",
  green: "#15803d",
  yellow: "#a16207",
  blue: "#1d4ed8",
  magenta: "#7e22ce",
  cyan: "#0e7490",
  white: "#52525b",
  brightBlack: "#71717a",
  brightRed: "#dc2626",
  brightGreen: "#16a34a",
  brightYellow: "#ca8a04",
  brightBlue: "#2563eb",
  brightMagenta: "#9333ea",
  brightCyan: "#0891b2",
  brightWhite: "#18181b",
};

export function swatchGradient(id: string): string {
  const pal = PALETTES[parseTheme(id)];
  return `linear-gradient(135deg, rgb(${pal.n["200"]}), rgb(${pal.n["950"]}))`;
}

export function applyDocumentTheme(id: string) {
  const key = parseTheme(id);
  const pal = PALETTES[key];
  if (!pal) return;
  const root = document.documentElement;
  root.dataset.theme = key;
  root.dataset.mode = pal.mode;
  for (const step of STEPS) {
    root.style.setProperty(`--n-${step}`, pal.n[step]);
  }
}

export function xtermTheme(id: ThemeId): ITheme {
  const pal = PALETTES[parseTheme(String(id))] ?? PALETTES.sage;
  const ansi = pal.mode === "dark" ? ANSI_DARK : ANSI_LIGHT;
  const background = rgbToHex(pal.n["950"]);
  const foreground = rgbToHex(pal.n["200"]);
  const cursor = rgbToHex(pal.n["600"]);
  const selection = rgbToHex(pal.n["700"]);
  return {
    ...ansi,
    background,
    foreground,
    cursor,
    cursorAccent: background,
    selectionBackground: selection,
    black: pal.mode === "dark" ? "#1c1c1c" : "#27272a",
  };
}

const CODE = new Set([
  "rust",
  "typescript",
  "tsx",
  "javascript",
  "jsx",
  "python",
  "go",
  "c",
  "cpp",
  "java",
  "kotlin",
  "swift",
  "ruby",
  "php",
  "csharp",
  "lua",
  "vim",
  "shell",
]);

export function isCodeLang(lang: string | null | undefined): boolean {
  return lang != null && CODE.has(lang);
}
