import { useId } from "react";
import catalog from "./vscodeIcons.json";

type Glyph = { body: string; width: number; height: number };

const ICONS = catalog as Record<string, Glyph>;

const BY_LANG: Record<string, string> = {
  rust: "file-type-rust",
  typescript: "file-type-typescript-official",
  tsx: "file-type-reactts",
  javascript: "file-type-js-official",
  jsx: "file-type-reactjs",
  python: "file-type-python",
  go: "file-type-go",
  c: "file-type-c",
  cpp: "file-type-cpp",
  java: "file-type-java",
  kotlin: "file-type-kotlin",
  swift: "file-type-swift",
  ruby: "file-type-ruby",
  php: "file-type-php",
  csharp: "file-type-csharp2",
  css: "file-type-css",
  scss: "file-type-scss",
  html: "file-type-html",
  json: "file-type-json",
  toml: "file-type-toml",
  yaml: "file-type-yaml",
  markdown: "file-type-markdown",
  shell: "file-type-shell",
  sql: "file-type-sql",
  lua: "file-type-lua",
  vim: "file-type-vim",
  xml: "file-type-xml",
  svg: "file-type-svg",
  docker: "file-type-docker",
  make: "file-type-gnu",
  cmake: "file-type-cmake",
};

function scopedBody(body: string, uid: string) {
  return body
    .replaceAll(/id="([^"]+)"/g, `id="${uid}$1"`)
    .replaceAll(/url\(#([^)]+)\)/g, `url(#${uid}$1)`);
}

function VscodeGlyph({ name, size }: { name: string; size: number }) {
  const uid = useId().replaceAll(":", "");
  const glyph = ICONS[name] ?? ICONS["default-file"];
  if (!glyph) return null;
  return (
    <svg
      className="file-icon"
      width={size}
      height={size}
      viewBox={`0 0 ${glyph.width} ${glyph.height}`}
      aria-hidden
      dangerouslySetInnerHTML={{ __html: scopedBody(glyph.body, uid) }}
    />
  );
}

export function FileTypeIcon({ lang, size = 16 }: { lang: string | null; size?: number }) {
  return <VscodeGlyph name={lang ? (BY_LANG[lang] ?? "default-file") : "default-file"} size={size} />;
}

export function FolderTypeIcon({ open, size = 16 }: { open: boolean; size?: number }) {
  return <VscodeGlyph name={open ? "default-folder-opened" : "default-folder"} size={size} />;
}
