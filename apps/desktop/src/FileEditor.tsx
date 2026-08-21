import { useEffect, useState } from "react";

type FileDoc = { rel: string; text: string; lang?: string | null };

export function FileEditor({
  rel,
  onOpenNvim,
}: {
  rel: string;
  onOpenNvim: () => void;
}) {
  const [doc, setDoc] = useState<FileDoc | null>(null);
  const [draft, setDraft] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const dirty = doc != null && draft !== doc.text;

  useEffect(() => {
    let alive = true;
    setErr(null);
    void window.lolterm.invoke("readFile", { rel }).then((value) => {
      if (!alive) return;
      if (value && typeof value === "object" && "text" in value) {
        const next = value as FileDoc;
        setDoc({ ...next, rel });
        setDraft(next.text);
      }
    }).catch((error: Error) => {
      if (alive) setErr(error.message);
    });
    return () => {
      alive = false;
    };
  }, [rel]);

  async function save() {
    await window.lolterm.invoke("writeFile", { rel, text: draft });
    setDoc((prev) => (prev ? { ...prev, text: draft } : prev));
  }

  return (
    <div className="doc-view">
      <div className="doc-bar">
        <span>{rel}</span>
        <span className="hint">{doc?.lang ?? "text"}</span>
        <button type="button" disabled={!dirty} onClick={() => void save()}>
          guardar
        </button>
        <button type="button" onClick={onOpenNvim}>
          nvim
        </button>
      </div>
      {err && <p className="quota-note">{err}</p>}
      <textarea
        className="doc-editor"
        value={draft}
        spellCheck={false}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if ((e.ctrlKey || e.metaKey) && e.key === "s") {
            e.preventDefault();
            void save();
          }
        }}
      />
    </div>
  );
}
