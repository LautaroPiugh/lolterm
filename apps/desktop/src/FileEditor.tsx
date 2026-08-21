import { useEffect, useState } from "react";
import CodeMirror, { EditorView } from "@uiw/react-codemirror";
import { languages } from "@codemirror/language-data";
import { LanguageDescription } from "@codemirror/language";
import type { Extension } from "@codemirror/state";

type FileDoc = { rel: string; text: string; lang?: string | null };

const cmTheme = EditorView.theme(
  {
    "&": {
      height: "100%",
      backgroundColor: "rgb(var(--n-950))",
      color: "rgb(var(--n-200))",
      fontSize: "13px",
    },
    ".cm-editor": {
      height: "100%",
      backgroundColor: "rgb(var(--n-950))",
      outline: "none",
    },
    ".cm-editor.cm-focused": { outline: "none" },
    ".cm-scroller": {
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      lineHeight: "1.55",
      overflow: "auto",
    },
    ".cm-content": { minHeight: "100%", padding: "10px 12px 20px" },
    ".cm-gutters": {
      backgroundColor: "rgb(var(--n-950))",
      border: "none",
      color: "rgb(var(--n-500))",
      paddingLeft: "8px",
    },
    ".cm-activeLine": { backgroundColor: "rgb(var(--n-900) / 0.55)" },
    ".cm-activeLineGutter": { backgroundColor: "rgb(var(--n-900) / 0.55)", color: "rgb(var(--n-300))" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "rgb(var(--n-700)) !important" },
    ".cm-cursor": { borderLeftColor: "rgb(var(--n-200))" },
  },
  { dark: true },
);

const cmThemeLight = EditorView.theme(
  {
    "&": {
      height: "100%",
      backgroundColor: "rgb(var(--n-950))",
      color: "rgb(var(--n-200))",
      fontSize: "13px",
    },
    ".cm-editor": {
      height: "100%",
      backgroundColor: "rgb(var(--n-950))",
      outline: "none",
    },
    ".cm-editor.cm-focused": { outline: "none" },
    ".cm-scroller": {
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      lineHeight: "1.55",
      overflow: "auto",
    },
    ".cm-content": { minHeight: "100%", padding: "10px 12px 20px" },
    ".cm-gutters": {
      backgroundColor: "rgb(var(--n-950))",
      border: "none",
      color: "rgb(var(--n-400))",
      paddingLeft: "8px",
    },
    ".cm-activeLine": { backgroundColor: "rgb(var(--n-800))" },
    ".cm-activeLineGutter": { backgroundColor: "rgb(var(--n-800))", color: "rgb(var(--n-300))" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "rgb(var(--n-700)) !important" },
    ".cm-cursor": { borderLeftColor: "rgb(var(--n-50))" },
  },
  { dark: false },
);

function fileName(rel: string) {
  const parts = rel.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || rel;
}

export function FileEditor({
  rel,
  theme,
  onOpenNvim,
  onDirtyChange,
  onSaved,
}: {
  rel: string;
  theme: string;
  onOpenNvim: () => void;
  onDirtyChange: (rel: string, dirty: boolean) => void;
  onSaved: () => void;
}) {
  const [doc, setDoc] = useState<FileDoc | null>(null);
  const [draft, setDraft] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [langExt, setLangExt] = useState<Extension[]>([]);
  const dirty = doc != null && draft !== doc.text;
  const dark = theme !== "claro";

  useEffect(() => {
    onDirtyChange(rel, dirty);
    return () => onDirtyChange(rel, false);
  }, [dirty, onDirtyChange, rel]);

  useEffect(() => {
    let alive = true;
    setErr(null);
    void window.lolterm
      .invoke("readFile", { rel })
      .then((value) => {
        if (!alive) return;
        if (value && typeof value === "object" && "text" in value) {
          const next = value as FileDoc;
          setDoc({ ...next, rel });
          setDraft(next.text);
        }
      })
      .catch((error: Error) => {
        if (alive) setErr(error.message);
      });
    return () => {
      alive = false;
    };
  }, [rel]);

  useEffect(() => {
    let alive = true;
    const description = LanguageDescription.matchFilename(languages, fileName(rel));
    if (!description) {
      setLangExt([]);
      return;
    }
    void description
      .load()
      .then((support) => {
        if (alive) setLangExt([support]);
      })
      .catch(() => {
        if (alive) setLangExt([]);
      });
    return () => {
      alive = false;
    };
  }, [rel]);

  async function save() {
    if (!dirty || saving) return;
    setSaving(true);
    setErr(null);
    try {
      await window.lolterm.invoke("writeFile", { rel, text: draft });
      setDoc((prev) => (prev ? { ...prev, text: draft } : prev));
      onSaved();
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="doc-view">
      <div className="doc-bar">
        <span>{rel}</span>
        <span className="hint">{doc?.lang ?? "text"}</span>
        {dirty && <span className="doc-dirty">sin guardar</span>}
        <button type="button" disabled={!dirty || saving} onClick={() => void save()}>
          {saving ? "guardando…" : "guardar"}
        </button>
        <button type="button" onClick={onOpenNvim}>
          nvim
        </button>
      </div>
      {err && <p className="quota-note">{err}</p>}
      <div
        className="doc-cm"
        onKeyDown={(e) => {
          if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
            e.preventDefault();
            void save();
          }
        }}
      >
        <CodeMirror
          value={draft}
          height="100%"
          theme={dark ? cmTheme : cmThemeLight}
          extensions={langExt}
          onChange={setDraft}
          basicSetup={{
            lineNumbers: true,
            foldGutter: true,
            highlightActiveLine: true,
            highlightActiveLineGutter: true,
            autocompletion: false,
          }}
        />
      </div>
    </div>
  );
}
