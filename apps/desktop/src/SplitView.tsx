import type { LayoutNode, PaneSnap } from "./types";
import { TerminalPane } from "./TerminalPane";

export function SplitView({
  node,
  panes,
  focused,
  onFocus,
}: {
  node: LayoutNode;
  panes: PaneSnap[];
  focused: number;
  onFocus: (id: number) => void;
}) {
  if (node.type === "leaf") {
    return (
      <TerminalPane
        pane={node.pane}
        focused={focused === node.pane}
        onFocus={() => onFocus(node.pane)}
      />
    );
  }
  const dir = node.dir === "columns" ? "row" : "column";
  return (
    <div className="split" style={{ flexDirection: dir }}>
      <div className="split-pane" style={{ flex: node.percent }}>
        <SplitView node={node.first} panes={panes} focused={focused} onFocus={onFocus} />
      </div>
      <div className={node.dir === "columns" ? "pane-divider" : "pane-divider-h"} />
      <div className="split-pane" style={{ flex: 100 - node.percent }}>
        <SplitView node={node.second} panes={panes} focused={focused} onFocus={onFocus} />
      </div>
    </div>
  );
}
