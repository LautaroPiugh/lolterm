import { useEffect, useRef, useState } from "react";
import { refitAllTerminals, TerminalPane } from "./TerminalPane";
import type { LayoutNode, PaneSnap } from "./types";

function firstLeafId(node: LayoutNode): number {
  return node.type === "leaf" ? node.pane : firstLeafId(node.first);
}

function clampPercent(value: number) {
  return Math.min(85, Math.max(15, Math.round(value)));
}

export function SplitView({
  node,
  panes,
  focused,
  zoomed,
  onFocus,
}: {
  node: LayoutNode;
  panes: PaneSnap[];
  focused: number;
  zoomed: number | null;
  onFocus: (id: number) => void;
}) {
  if (zoomed != null && panes.some((pane) => pane.id === zoomed)) {
    return (
      <TerminalPane
        pane={zoomed}
        focused={focused === zoomed}
        onFocus={() => onFocus(zoomed)}
      />
    );
  }
  if (node.type === "leaf") {
    return (
      <TerminalPane
        pane={node.pane}
        focused={focused === node.pane}
        onFocus={() => onFocus(node.pane)}
      />
    );
  }
  return <SplitPair node={node} panes={panes} focused={focused} onFocus={onFocus} />;
}

function SplitPair({
  node,
  panes,
  focused,
  onFocus,
}: {
  node: Extract<LayoutNode, { type: "split" }>;
  panes: PaneSnap[];
  focused: number;
  onFocus: (id: number) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [percent, setPercent] = useState(node.percent);
  const percentRef = useRef(percent);
  const dragRef = useRef(false);
  const [drag, setDrag] = useState(false);

  percentRef.current = percent;

  useEffect(() => {
    if (!dragRef.current) setPercent(node.percent);
  }, [node.percent]);

  const columns = node.dir === "columns";

  function onPointerDown(e: React.PointerEvent<HTMLDivElement>) {
    e.preventDefault();
    dragRef.current = true;
    setDrag(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: React.PointerEvent<HTMLDivElement>) {
    if (!dragRef.current || !host.current) return;
    const rect = host.current.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return;
    const ratio = columns
      ? (e.clientX - rect.left) / rect.width
      : (e.clientY - rect.top) / rect.height;
    setPercent(clampPercent(ratio * 100));
  }

  function onPointerUp() {
    if (!dragRef.current) return;
    dragRef.current = false;
    setDrag(false);
    void window.lolterm.invoke("setSplit", {
      pane: firstLeafId(node.first),
      other: firstLeafId(node.second),
      percent: percentRef.current,
    });
    refitAllTerminals();
  }

  return (
    <div
      className={drag ? "split dragging" : "split"}
      ref={host}
      style={{ flexDirection: columns ? "row" : "column" }}
    >
      <div className="split-pane" style={{ flex: percent }}>
        <SplitView node={node.first} panes={panes} focused={focused} onFocus={onFocus} />
      </div>
      <div
        className={columns ? "pane-divider" : "pane-divider-h"}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
      />
      <div className="split-pane" style={{ flex: 100 - percent }}>
        <SplitView node={node.second} panes={panes} focused={focused} onFocus={onFocus} />
      </div>
    </div>
  );
}
