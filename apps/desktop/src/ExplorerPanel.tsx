import { useEffect, useState, type KeyboardEvent } from "react";
import {
  ChevronDown,
  ChevronRight,
  FilePlus,
  FolderPlus,
  Pencil,
  RotateCcw,
  Trash2,
} from "./icons";
import { FileTypeIcon, FolderTypeIcon } from "./fileIcons";
import type { TreeRow } from "./types";

type Menu = { x: number; y: number; rel: string; isDir: boolean };
type Draft = { kind: "file" | "dir" | "rename"; rel: string; name: string };

function fileGlyph(row: TreeRow) {
  if (row.is_dir) return <FolderTypeIcon open={row.expanded} size={16} />;
  return <FileTypeIcon lang={row.lang} size={16} />;
}

function badgeClass(mark: string | null) {
  if (mark === "M") return "m";
  if (mark === "A") return "a";
  if (mark === "?") return "u";
  return "u";
}

function parentRel(rel: string) {
  const cut = rel.lastIndexOf("/");
  return cut < 0 ? "" : rel.slice(0, cut);
}

export function ExplorerPanel({
  tree,
  activeRel,
  onSearch,
  call,
}: {
  tree: TreeRow[];
  activeRel?: string | null;
  onSearch: () => void;
  call: (method: string, params?: unknown) => Promise<unknown>;
}) {
  const [menu, setMenu] = useState<Menu | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [selected, setSelected] = useState<string>("");

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [menu]);

  function startCreate(parent: string, kind: "file" | "dir") {
    setMenu(null);
    setDraft({ kind, rel: parent, name: "" });
  }

  function startRename(rel: string, name: string) {
    setMenu(null);
    setDraft({ kind: "rename", rel, name });
  }

  async function submitDraft() {
    if (!draft) return;
    const current = draft;
    const name = current.name.trim();
    setDraft(null);
    if (!name) return;
    if (current.kind === "rename") {
      await call("fsOp", { op: "rename", rel: current.rel, name });
    } else {
      await call("fsOp", {
        op: current.kind === "dir" ? "createDir" : "createFile",
        rel: current.rel,
        name,
      });
    }
  }

  async function remove(rel: string, isDir: boolean) {
    setMenu(null);
    const label = rel || "/";
    if (!window.confirm(isDir ? `¿Borrar la carpeta ${label} y su contenido?` : `¿Borrar ${label}?`)) {
      return;
    }
    await call("fsOp", { op: "delete", rel });
  }

  function onTreeKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
    const row = tree.find((item) => item.rel === selected);
    if (e.key === "F2" && row && row.rel) {
      e.preventDefault();
      startRename(row.rel, row.name);
    }
    if ((e.key === "Delete" || e.key === "Backspace") && row && row.rel) {
      e.preventDefault();
      void remove(row.rel, row.is_dir);
    }
  }

  return (
    <>
      <div className="sidebar-tabs">
        <span className="stab on">Files</span>
        <button type="button" className="stab" onClick={onSearch}>
          Search
        </button>
        <span className="stab-actions">
          <button type="button" className="stab-icon" title="Nuevo archivo" onClick={() => startCreate(selected && tree.find((r) => r.rel === selected)?.is_dir ? selected : parentRel(selected), "file")}>
            <FilePlus size={12} />
          </button>
          <button type="button" className="stab-icon" title="Nueva carpeta" onClick={() => startCreate(selected && tree.find((r) => r.rel === selected)?.is_dir ? selected : parentRel(selected), "dir")}>
            <FolderPlus size={12} />
          </button>
          <button type="button" className="stab-icon" title="Actualizar" onClick={() => void call("fsOp", { op: "refresh", rel: "" })}>
            <RotateCcw size={12} />
          </button>
        </span>
      </div>
      <div
        className="sidebar-content"
        onKeyDown={onTreeKey}
        onContextMenu={(e) => {
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, rel: "", isDir: true });
        }}
      >
        {draft && draft.kind !== "rename" && (
          <input
            className="tree-create"
            autoFocus
            placeholder={draft.kind === "dir" ? "carpeta…" : "archivo…"}
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            onBlur={() => void submitDraft()}
            onKeyDown={(e) => {
              if (e.key === "Enter") void submitDraft();
              if (e.key === "Escape") setDraft(null);
            }}
            onClick={(e) => e.stopPropagation()}
          />
        )}
        {tree.map((row) => {
          const on = row.rel === activeRel || row.rel === selected;
          if (draft?.kind === "rename" && draft.rel === row.rel) {
            return (
              <input
                key={row.rel || "/"}
                className="tree-create"
                style={{ marginLeft: 8 + row.depth * 16 }}
                autoFocus
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                onBlur={() => void submitDraft()}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void submitDraft();
                  if (e.key === "Escape") setDraft(null);
                }}
                onClick={(e) => e.stopPropagation()}
              />
            );
          }
          return (
            <button
              key={row.rel || "/"}
              type="button"
              className={`${row.hidden ? "tree-item hidden" : "tree-item"}${on ? " on" : ""}`}
              title={row.hidden ? `${row.name} (oculto)` : (row.lang ?? undefined)}
              style={{ paddingLeft: 8 + row.depth * 16 }}
              onClick={() => {
                setSelected(row.rel);
                if (row.is_dir) void call("toggleExpand", { rel: row.rel });
                else void call("openFile", { rel: row.rel });
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setSelected(row.rel);
                setMenu({ x: e.clientX, y: e.clientY, rel: row.rel, isDir: row.is_dir });
              }}
            >
              {row.is_dir ? (
                row.expanded ? (
                  <ChevronDown size={10} color="var(--muted)" />
                ) : (
                  <ChevronRight size={10} color="var(--muted)" />
                )
              ) : (
                <span style={{ width: 10, flexShrink: 0 }} />
              )}
              <span style={{ marginLeft: 4, flexShrink: 0 }}>{fileGlyph(row)}</span>
              <span className="tree-name" style={{ marginLeft: 5 }}>
                {row.name}
              </span>
              {row.mark && <span className={`tree-badge ${badgeClass(row.mark)}`}>{row.mark}</span>}
            </button>
          );
        })}
      </div>
      {menu && (
        <div className="tree-menu" style={{ left: menu.x, top: menu.y }} onClick={(e) => e.stopPropagation()}>
          <button type="button" onClick={() => startCreate(menu.isDir ? menu.rel : parentRel(menu.rel), "file")}>
            <FilePlus size={12} /> Nuevo archivo
          </button>
          <button type="button" onClick={() => startCreate(menu.isDir ? menu.rel : parentRel(menu.rel), "dir")}>
            <FolderPlus size={12} /> Nueva carpeta
          </button>
          {menu.rel !== "" && (
            <>
              <button type="button" onClick={() => startRename(menu.rel, menu.rel.split("/").pop() || menu.rel)}>
                <Pencil size={12} /> Renombrar
              </button>
              <button type="button" className="danger" onClick={() => void remove(menu.rel, menu.isDir)}>
                <Trash2 size={12} /> Borrar
              </button>
            </>
          )}
        </div>
      )}
    </>
  );
}
