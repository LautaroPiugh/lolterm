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
 * Cuatro paletas independientes (no son pares claro/oscuro del mismo tinte).
 * La escala sigue el contrato de Orquester: 11 pasos RGB, 50 = texto, 950 = fondo.
 */
export const PALETTES: Record<string, Palette> = {
  claro: {
    mode: "light",
    n: scale(
      "18 22 28",
      "32 38 46",
      "48 56 66",
      "70 80 92",
      "98 110 124",
      "122 134 148",
      "148 160 174",
      "206 216 226",
      "226 232 238",
      "242 246 250",
      "252 253 254",
    ),
  },
  oscuro: {
    mode: "dark",
    n: scale(
      "236 238 236",
      "214 218 214",
      "180 188 182",
      "142 152 146",
      "104 116 110",
      "72 84 78",
      "50 62 56",
      "34 44 40",
      "22 30 26",
      "13 17 15",
      "7 9 8",
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
  { id: "claro", label: "Claro", hint: "papel frío" },
  { id: "oscuro", label: "Oscuro", hint: "carbón" },
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
  const pal = PALETTES[parseTheme(String(id))] ?? PALETTES.claro;
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
