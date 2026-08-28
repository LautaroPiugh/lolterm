import { useState } from "react";
import { writeClipboard } from "./copyOnSelect";
import { AlertTriangle, Copy, X } from "./icons";
import { clearDiag, formatReport, type DiagEntry } from "./diagnostics";

export function DiagnosticsPanel({
  entries,
  version,
  theme,
  root,
  onChange,
  onClose,
}: {
  entries: DiagEntry[];
  version: string;
  theme: string;
  root: string;
  onChange: (rows: DiagEntry[]) => void;
  onClose: () => void;
}) {
  const report = formatReport(entries, { version, theme, root });
  const [copyHint, setCopyHint] = useState<"ok" | "err" | null>(null);

  async function copy() {
    try {
      await writeClipboard(report);
      setCopyHint("ok");
      window.setTimeout(() => setCopyHint(null), 1800);
    } catch {
      setCopyHint("err");
    }
  }

  async function openIssue() {
    await copy();
    const title = encodeURIComponent(entries[0] ? `error: ${entries[0].source}` : "informe de error");
    const body = encodeURIComponent(`<!-- pegá el informe copiado debajo si el cuerpo queda corto -->\n\n\`\`\`\n${report.slice(0, 3500)}\n\`\`\`\n`);
    await window.lolterm.openExternal?.(
      `https://github.com/LautaroPiugh/lolterm/issues/new?title=${title}&body=${body}`,
    );
  }

  return (
    <div className="settings-panel diag-panel" onClick={(e) => e.stopPropagation()}>
      <header className="settings-head">
        <AlertTriangle size={14} color="var(--err)" />
        <div>
          <h2>Diagnóstico</h2>
          <p className="settings-kicker">log local · no se envía solo</p>
        </div>
        <button type="button" className="settings-close" title="Cerrar" onClick={onClose}>
          <X size={14} />
        </button>
      </header>
      <p className="settings-lead">
        LoLTerm guarda avisos del sidecar y de IPC en este equipo. Copiá el informe o abrí un issue en GitHub; no hay
        telemetría. Revisá el texto antes de publicarlo.
      </p>
      <div className="settings-row-actions diag-actions">
        <button type="button" className="settings-ghost" onClick={() => void copy()}>
          <Copy size={12} />
          Copiar informe
        </button>
        {copyHint === "ok" && (
          <span className="diag-copied" role="status">
            Informe copiado
          </span>
        )}
        {copyHint === "err" && (
          <span className="diag-copied is-err" role="status">
            No se pudo copiar
          </span>
        )}
        <button type="button" className="settings-ghost" onClick={() => void openIssue()}>
          Abrir issue en GitHub
        </button>
        <button type="button" className="settings-ghost" disabled={entries.length === 0} onClick={() => onChange(clearDiag())}>
          Vaciar
        </button>
      </div>
      {entries.length === 0 ? (
        <p className="welcome-empty">sin errores recientes</p>
      ) : (
        <ol className="diag-list">
          {entries.map((row) => (
            <li key={`${row.ts}-${row.source}`} className={`diag-row is-${row.kind}`}>
              <span className="diag-when">{new Date(row.ts).toLocaleString()}</span>
              <strong>{row.source}</strong>
              <span>{row.message}</span>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
