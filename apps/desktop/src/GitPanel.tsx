import { useState, type ReactNode } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  GitBranch,
  GitCommitHorizontal,
  Minus,
  Plus,
  RotateCcw,
} from "./icons";
import type { GitFile, Snapshot } from "./types";

type Call = (method: string, params?: unknown) => Promise<unknown>;

export function GitPanel({ snap, call }: { snap: Snapshot; call: Call }) {
  const files = snap.git_files ?? [];
  const staged = files.filter((file) => file.staged);
  const changes = files.filter((file) => file.unstaged || file.untracked);
  const git = snap.git;
  const [message, setMessage] = useState("");
  const [open, setOpen] = useState({ staged: true, changes: true, branches: false, log: true });

  const canCommit = message.trim().length > 0 && staged.length > 0;

  function toggle(key: keyof typeof open) {
    setOpen((prev) => ({ ...prev, [key]: !prev[key] }));
  }

  function op(name: string, extra?: Record<string, string>) {
    void call("gitOp", { op: name, ...extra });
  }

  function commit() {
    const text = message.trim();
    if (!text || staged.length === 0) return;
    void call("gitOp", { op: "commit", message: text }).then(() => setMessage(""));
  }

  function discard(file: GitFile) {
    if (file.untracked) return;
    if (!window.confirm(`¿Descartar cambios en ${file.path}?`)) return;
    op("discard", { path: file.path });
  }

  return (
    <>
      <div className="git-branch-bar">
        <GitBranch size={12} color="var(--muted)" />
        <span className="branch-name">{git?.branch ?? "sin repo"}</span>
        {git && (git.ahead > 0 || git.behind > 0) ? (
          <span className="git-sync-hint">
            {git.ahead > 0 ? `↑${git.ahead}` : null}
            {git.behind > 0 ? ` ↓${git.behind}` : null}
          </span>
        ) : null}
        <button type="button" className="lazygit-btn" onClick={() => void call("run", { program: "lazygit", args: [] })}>
          lazygit
        </button>
      </div>
      <div className="sidebar-content git-scm">
        {!git ? (
          <p className="hint">este workspace no es un repositorio git</p>
        ) : (
          <>
            <div className="git-commit-box">
              <textarea
                className="git-commit-msg"
                rows={3}
                value={message}
                placeholder="Mensaje (Ctrl+Enter para commit)"
                onChange={(e) => setMessage(e.target.value)}
                onKeyDown={(e) => {
                  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
                    e.preventDefault();
                    commit();
                  }
                }}
              />
              <div className="git-commit-row">
                <button type="button" className="git-commit-btn" disabled={!canCommit} onClick={commit} title="Commit (Ctrl+Enter)">
                  <Check size={13} strokeWidth={2.4} />
                  Commit
                </button>
                <button type="button" className="git-text-btn" onClick={() => op("fetch")}>
                  fetch
                </button>
                <button type="button" className="git-text-btn" onClick={() => op("pull")} title="git pull --ff-only">
                  pull
                </button>
              </div>
              {staged.length === 0 && changes.length > 0 ? (
                <p className="hint">stage archivos para poder commitear</p>
              ) : null}
            </div>

            <GitSection
              title="Staged Changes"
              count={staged.length}
              open={open.staged}
              onToggle={() => toggle("staged")}
              action={
                staged.length > 0 ? (
                  <button type="button" className="git-icon-btn" title="Unstage all" onClick={() => op("unstage")}>
                    <Minus size={12} />
                  </button>
                ) : null
              }
            >
              {staged.length === 0 ? <p className="hint git-empty">ninguno</p> : null}
              {staged.map((file) => (
                <GitFileRow
                  key={`s:${file.path}`}
                  file={file}
                  side="index"
                  onOpen={() => void call("openFile", { rel: file.path })}
                  onPrimary={() => op("unstage", { path: file.path })}
                  primaryTitle="Unstage"
                  primaryIcon="minus"
                />
              ))}
            </GitSection>

            <GitSection
              title="Changes"
              count={changes.length}
              open={open.changes}
              onToggle={() => toggle("changes")}
              action={
                changes.length > 0 ? (
                  <button type="button" className="git-icon-btn" title="Stage all" onClick={() => op("stage")}>
                    <Plus size={12} />
                  </button>
                ) : null
              }
            >
              {changes.length === 0 && staged.length === 0 ? <p className="hint git-empty">limpio</p> : null}
              {changes.map((file) => (
                <GitFileRow
                  key={`c:${file.path}`}
                  file={file}
                  side="work"
                  onOpen={() => void call("openFile", { rel: file.path })}
                  onPrimary={() => op("stage", { path: file.path })}
                  primaryTitle="Stage"
                  primaryIcon="plus"
                  onDiscard={file.untracked ? undefined : () => discard(file)}
                />
              ))}
            </GitSection>

            <GitSection title="Ramas" count={snap.git_branches?.length ?? 0} open={open.branches} onToggle={() => toggle("branches")}>
              {(snap.git_branches ?? []).map((branch) => (
                <button
                  key={branch}
                  type="button"
                  className={branch === git.branch ? "git-branch-hit current" : "git-branch-hit"}
                  title={`checkout ${branch}`}
                  onClick={() => {
                    if (branch === git.branch) return;
                    if (!window.confirm(`¿Cambiar a ${branch}?`)) return;
                    op("checkout", { path: branch });
                  }}
                >
                  {branch}
                </button>
              ))}
            </GitSection>

            <GitSection title="Commits" count={snap.git_log.length} open={open.log} onToggle={() => toggle("log")}>
              {snap.git_log.map((line) => {
                const sha = line.slice(0, 7);
                const msg = line.slice(8);
                return (
                  <div key={line} className="git-log-sidebar-item">
                    <GitCommitHorizontal size={11} color="var(--ok)" />
                    <span className="git-sha">{sha}</span>
                    <span className="git-log-msg">{msg || line}</span>
                  </div>
                );
              })}
            </GitSection>
          </>
        )}
      </div>
    </>
  );
}

function GitSection({
  title,
  count,
  open,
  onToggle,
  action,
  children,
}: {
  title: string;
  count: number;
  open: boolean;
  onToggle: () => void;
  action?: ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="git-section">
      <div className="git-section-head">
        <button type="button" className="git-section-toggle" onClick={onToggle}>
          {open ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
          <span>
            {title}
            {count > 0 ? <span className="git-count">{count}</span> : null}
          </span>
        </button>
        {action}
      </div>
      {open ? children : null}
    </section>
  );
}

function GitFileRow({
  file,
  side,
  onOpen,
  onPrimary,
  primaryTitle,
  primaryIcon,
  onDiscard,
}: {
  file: GitFile;
  side: "index" | "work";
  onOpen: () => void;
  onPrimary: () => void;
  primaryTitle: string;
  primaryIcon: "plus" | "minus";
  onDiscard?: () => void;
}) {
  const letter = statusLetter(file, side);
  const { name, dir } = splitPath(file.path);
  return (
    <div className="git-file-row">
      <button type="button" className="git-file-name" title={file.path} onClick={onOpen}>
        <span className="git-file-base">{name}</span>
        {dir ? <span className="git-file-dir">{dir}</span> : null}
      </button>
      <span className="git-file-actions">
        {onDiscard ? (
          <button type="button" className="git-icon-btn" title="Discard" onClick={onDiscard}>
            <RotateCcw size={11} />
          </button>
        ) : null}
        <button type="button" className="git-icon-btn" title={primaryTitle} onClick={onPrimary}>
          {primaryIcon === "plus" ? <Plus size={12} /> : <Minus size={12} />}
        </button>
      </span>
      <span className={`git-letter git-letter-${letter.toLowerCase()}`}>{letter}</span>
    </div>
  );
}

function splitPath(path: string) {
  const slash = path.lastIndexOf("/");
  if (slash < 0) return { name: path, dir: "" };
  return { name: path.slice(slash + 1), dir: path.slice(0, slash) };
}

/** Porcelain XY: X = índice (staged), Y = working tree. VS Code muestra una sola letra. */
export function statusLetter(file: GitFile, side: "index" | "work"): string {
  if (file.untracked) return "U";
  const x = file.mark[0] ?? " ";
  const y = file.mark[1] ?? " ";
  const ch = side === "index" ? x : y;
  if (ch === " " || ch === "?") return x !== " " && x !== "?" ? x : "M";
  return ch;
}
