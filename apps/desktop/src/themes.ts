import type { ITheme } from "@xterm/xterm";

export type ThemeId = "sage" | "dusk" | "mono";

export const THEMES: { id: ThemeId; label: string; hint: string }[] = [
  { id: "sage", label: "Sage", hint: "mint claro" },
  { id: "dusk", label: "Dusk", hint: "oscuro" },
  { id: "mono", label: "Mono", hint: "gris" },
];

export function parseTheme(value: string | null | undefined): ThemeId {
  if (value === "dusk" || value === "mono" || value === "sage") return value;
  return "sage";
}

export function xtermTheme(id: ThemeId): ITheme {
  if (id === "dusk") {
    return {
      background: "#161a14",
      foreground: "#e4ebe4",
      cursor: "#6aaf78",
      cursorAccent: "#161a14",
      selectionBackground: "#3d5c42",
      black: "#1c2018",
      red: "#e07070",
      green: "#6aaf78",
      yellow: "#c4b070",
      blue: "#7a9cb8",
      magenta: "#b890b0",
      cyan: "#7ab8a8",
      white: "#e4ebe4",
      brightBlack: "#6a7a6c",
      brightRed: "#f09090",
      brightGreen: "#8fd49a",
      brightYellow: "#dcc888",
      brightBlue: "#9cb8d0",
      brightMagenta: "#d0a8c8",
      brightCyan: "#98d0c0",
      brightWhite: "#f4f7f4",
    };
  }
  if (id === "mono") {
    return {
      background: "#fafaf8",
      foreground: "#1a1a1a",
      cursor: "#333333",
      cursorAccent: "#fafaf8",
      selectionBackground: "#cfcfc8",
      black: "#1a1a1a",
      red: "#8a2020",
      green: "#2a2a2a",
      yellow: "#555555",
      blue: "#444444",
      magenta: "#555555",
      cyan: "#444444",
      white: "#f2f2f0",
      brightBlack: "#6a6a6a",
      brightRed: "#a04040",
      brightGreen: "#404040",
      brightYellow: "#777777",
      brightBlue: "#555555",
      brightMagenta: "#666666",
      brightCyan: "#555555",
      brightWhite: "#ffffff",
    };
  }
  return {
    background: "#f4f7f4",
    foreground: "#28302a",
    cursor: "#488c58",
    cursorAccent: "#f4f7f4",
    selectionBackground: "#a8d4b0",
    black: "#28302a",
    red: "#b04040",
    green: "#488c58",
    yellow: "#8a7a30",
    blue: "#3a6a88",
    magenta: "#7a5a78",
    cyan: "#3a7a70",
    white: "#ecf2ec",
    brightBlack: "#6c8070",
    brightRed: "#d06060",
    brightGreen: "#6aa878",
    brightYellow: "#b0a050",
    brightBlue: "#5a8aa8",
    brightMagenta: "#9a7a98",
    brightCyan: "#5a9a90",
    brightWhite: "#ffffff",
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
