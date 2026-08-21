import { useEffect, useState } from "react";

type RestResult = { ok: boolean; status: string; headers: string; body: string };

export function RestClient({ rel }: { rel: string }) {
  const [text, setText] = useState("");
  const [out, setOut] = useState<RestResult | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    void window.lolterm.invoke("readFile", { rel }).then((value) => {
      if (!alive) return;
      if (value && typeof value === "object" && "text" in value) {
        setText((value as { text: string }).text);
      }
    });
    return () => {
      alive = false;
    };
  }, [rel]);

  async function saveAndSend() {
    setBusy(true);
    setErr(null);
    try {
      await window.lolterm.invoke("writeFile", { rel, text });
      const result = await window.lolterm.invoke("restSend", { rel });
      setOut(result as RestResult);
    } catch (error) {
      setErr(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="doc-view rest-view">
      <div className="doc-bar">
        <span>{rel}</span>
        <span className="hint">curl · variables de .env</span>
        <button type="button" disabled={busy} onClick={() => void saveAndSend()}>
          {busy ? "…" : "enviar"}
        </button>
      </div>
      <textarea className="doc-editor" value={text} spellCheck={false} onChange={(e) => setText(e.target.value)} />
      {err && <pre className="rest-out err">{err}</pre>}
      {out && (
        <pre className="rest-out">
          {out.status}
          {"\n"}
          {out.headers}
          {"\n\n"}
          {out.body}
        </pre>
      )}
    </div>
  );
}
