import type { ITheme } from "@xterm/xterm";

/** Escala Orquester: 50 texto → 950 fondo (se invierte en oscuro). */
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

/**
 * Paletas independientes (no son pares claro/oscuro del mismo tinte).
 * La escala: 11 pasos RGB, 50 = texto, 950 = fondo.
 */
export const PALETTES: Record<string, Palette> = {
  claro: {
    mode: "light",
    // chrome #F3F3F3, editor #FCFCFC, texto #141414
    n: scale(
      "20 20 20",
      "20 20 20",
      "55 55 55",
      "80 80 80",
      "110 110 110",
      "140 140 140",
      "39 120 193",
      "228 228 228",
      "243 243 243",
      "243 243 243",
      "252 252 252",
    ),
  },
  oscuro: {
    mode: "dark",
    // chrome #141414, editor #181818, acento #81A1C1
    n: scale(
      "255 255 255",
      "240 240 240",
      "220 220 220",
      "177 177 177",
      "160 160 160",
      "90 90 90",
      "129 161 193",
      "36 36 36",
      "20 20 20",
      "20 20 20",
      "24 24 24",
    ),
  },
  contraste: {
    mode: "dark",
    // negro #0A0A0A, texto #F0F0F0
    n: scale(
      "255 255 255",
      "240 240 240",
      "220 220 220",
      "177 177 177",
      "160 160 160",
      "88 88 88",
      "67 76 94",
      "42 42 42",
      "10 10 10",
      "10 10 10",
      "10 10 10",
    ),
  },
  tide: {
    mode: "dark",
    n: scale(
      "226 242 244",
      "196 228 234",
      "154 206 218",
      "108 176 194",
      "68 142 164",
      "36 108 132",
      "22 78 100",
      "14 56 74",
      "10 40 54",
      "7 28 40",
      "4 16 24",
    ),
  },
  ember: {
    mode: "light",
    n: scale(
      "36 20 12",
      "56 32 18",
      "82 48 26",
      "112 68 38",
      "148 96 52",
      "176 120 64",
      "196 142 82",
      "232 198 152",
      "244 222 192",
      "252 240 224",
      "255 250 244",
    ),
  },
};

export type ThemeId = keyof typeof PALETTES | string;

export const THEMES: { id: string; label: string; hint: string }[] = [
  { id: "claro", label: "Claro", hint: "papel, chrome gris" },
  { id: "oscuro", label: "Oscuro", hint: "carbón #141414" },
  { id: "contraste", label: "Contraste", hint: "negro puro" },
  { id: "tide", label: "Tide", hint: "mar de noche" },
  { id: "ember", label: "Ember", hint: "cobre" },
];

export function isBuiltinTheme(id: string): boolean {
  return Object.hasOwn(PALETTES, id);
}

export function parseTheme(value: string | null | undefined): string {
  if (value && isBuiltinTheme(value)) return value;
  return "claro";
}

const THEME_STORE = "lolterm.theme";
const FILL_STORE = "lolterm.fill";

const THEME_FILL: Record<string, string> = {
  claro: "#f3f3f3",
  oscuro: "#141414",
  contraste: "#0a0a0a",
  tide: "#071c28",
  ember: "#fcf0e0",
};

export function chromeFill(id: string): string {
  return THEME_FILL[parseTheme(id)] ?? THEME_FILL.claro;
}

export function rememberedTheme(): string {
  try {
    return parseTheme(localStorage.getItem(THEME_STORE));
  } catch {
    return "claro";
  }
}

function persistChrome(id: string) {
  const key = parseTheme(id);
  try {
    localStorage.setItem(THEME_STORE, key);
    localStorage.setItem(FILL_STORE, chromeFill(key));
  } catch {
    // storage lleno o bloqueado; el siguiente arranque usa claro
  }
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
  for (const name of ["fill", "text", "brand", "bar", "pane", "muted", "focus", "border", "err", "ok", "hover"]) {
    root.style.removeProperty(`--${name}`);
  }
  for (const step of STEPS) {
    root.style.setProperty(`--n-${step}`, pal.n[step]);
  }
  persistChrome(key);
}

export function xtermTheme(id: ThemeId): ITheme {
  const key = parseTheme(String(id));
  if (key === "oscuro") {
    return XTERM_OSCURO;
  }
  if (key === "contraste") {
    return XTERM_CONTRASTE;
  }
  if (key === "claro") {
    return XTERM_CLARO;
  }
  if (key === "tide") {
    return XTERM_TIDE;
  }
  if (key === "ember") {
    return XTERM_EMBER;
  }
  const pal = PALETTES[key] ?? PALETTES.claro;
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

const XTERM_OSCURO: ITheme = {
  background: "#181818",
  foreground: "#F0F0F0",
  cursor: "#F0F0F0",
  cursorAccent: "#181818",
  selectionBackground: "#40404099",
  black: "#242424",
  red: "#FC6B83",
  green: "#3FA266",
  yellow: "#D2943E",
  blue: "#81A1C1",
  magenta: "#B48EAD",
  cyan: "#88C0D0",
  white: "#F0F0F0",
  brightBlack: "#888888",
  brightRed: "#FC6B83",
  brightGreen: "#70B489",
  brightYellow: "#F1B467",
  brightBlue: "#87A6C4",
  brightMagenta: "#B48EAD",
  brightCyan: "#88C0D0",
  brightWhite: "#FFFFFF",
};

const XTERM_CONTRASTE: ITheme = {
  background: "#0A0A0A",
  foreground: "#F0F0F0",
  cursor: "#F0F0F0",
  cursorAccent: "#0A0A0A",
  selectionBackground: "#40404099",
  black: "#2A2A2A",
  red: "#BF616A",
  green: "#A3BE8C",
  yellow: "#EBCB8B",
  blue: "#81A1C1",
  magenta: "#B48EAD",
  cyan: "#88C0D0",
  white: "#F0F0F0",
  brightBlack: "#888888",
  brightRed: "#BF616A",
  brightGreen: "#A3BE8C",
  brightYellow: "#EBCB8B",
  brightBlue: "#81A1C1",
  brightMagenta: "#B48EAD",
  brightCyan: "#88C0D0",
  brightWhite: "#FFFFFF",
};

const XTERM_CLARO: ITheme = {
  background: "#FCFCFC",
  foreground: "#141414",
  cursor: "#141414",
  cursorAccent: "#FCFCFC",
  selectionBackground: "#14141414",
  black: "#141414",
  red: "#BE1744",
  green: "#007041",
  yellow: "#8B5700",
  blue: "#0064B0",
  magenta: "#92156A",
  cyan: "#176C74",
  white: "#FCFCFC",
  brightBlack: "#6B6B6B",
  brightRed: "#CE405B",
  brightGreen: "#00854C",
  brightYellow: "#A46700",
  brightBlue: "#2778C1",
  brightMagenta: "#B54E90",
  brightCyan: "#3B7E84",
  brightWhite: "#FFFFFF",
};

const XTERM_TIDE: ITheme = {
  background: "#041018",
  foreground: "#c4e4ea",
  cursor: "#6cb0c2",
  cursorAccent: "#041018",
  selectionBackground: "#0e384a",
  black: "#071c28",
  red: "#f87171",
  green: "#5eead4",
  yellow: "#fde68a",
  blue: "#6cb0c2",
  magenta: "#c4b5fd",
  cyan: "#9aceda",
  white: "#e2f2f4",
  brightBlack: "#448ea4",
  brightRed: "#fca5a5",
  brightGreen: "#99f6e4",
  brightYellow: "#fef3c7",
  brightBlue: "#9aceda",
  brightMagenta: "#ddd6fe",
  brightCyan: "#c4e4ea",
  brightWhite: "#ffffff",
};

const XTERM_EMBER: ITheme = {
  background: "#fffaf4",
  foreground: "#382012",
  cursor: "#c48e52",
  cursorAccent: "#fffaf4",
  selectionBackground: "#e8c698",
  black: "#382012",
  red: "#9a3412",
  green: "#3f6212",
  yellow: "#a16207",
  blue: "#9a3412",
  magenta: "#9f1239",
  cyan: "#0f766e",
  white: "#fcf0e0",
  brightBlack: "#946034",
  brightRed: "#c2410c",
  brightGreen: "#4d7c0f",
  brightYellow: "#ca8a04",
  brightBlue: "#b45309",
  brightMagenta: "#be123c",
  brightCyan: "#0d9488",
  brightWhite: "#fffaf4",
};

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
