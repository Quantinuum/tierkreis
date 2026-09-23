import { Input } from "@/components/ui/input";
import { BackendNode } from "@/nodes/types";
import { useMemo, useState } from "react";
import { useReactFlow, type Edge } from "@xyflow/react";

// Client-side search over the nodes already loaded for the current graph view.
export const NodeSearch = (props: { nodes: BackendNode[] }) => {
  const reactFlowInstance = useReactFlow<BackendNode, Edge>();
  const [query, setQuery] = useState("");

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return props.nodes.filter(
      (n) =>
        (n.data.name || "").toLowerCase().includes(q) ||
        (n.data.title || "").toLowerCase().includes(q) ||
        n.data.node_location.toLowerCase().includes(q),
    );
  }, [props.nodes, query]);

  const jumpTo = (node: BackendNode) => {
    reactFlowInstance.setNodes((nds) =>
      nds.map((n) => ({ ...n, selected: n.id === node.id })),
    );
    reactFlowInstance.fitView({ nodes: [{ id: node.id }], duration: 300 });
  };

  return (
    <div className="relative w-64">
      <Input
        placeholder="Search nodes by name or location..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="bg-background"
      />
      {matches.length > 0 && (
        <div className="absolute mt-1 w-full max-h-64 overflow-auto rounded-md border bg-popover shadow-md z-20">
          {matches.map((n) => (
            <button
              key={n.id}
              className="w-full text-left px-3 py-2 text-sm hover:bg-accent cursor-pointer"
              onClick={() => jumpTo(n)}
            >
              <div className="font-medium truncate">
                {n.data.name || n.data.title}
              </div>
              <div className="text-xs text-muted-foreground truncate">
                {n.data.node_location} &middot; {n.data.status}
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
};
