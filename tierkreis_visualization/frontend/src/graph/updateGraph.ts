import { BackendNode } from "@/nodes/types";
import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "@/routes/workflows/_.$wid.nodes.$loc/-components/models";
import { Rect, XYPosition } from "@xyflow/react";

const positionInRect = (p: XYPosition, rect: Rect): boolean => {
  const x_in = rect.x <= p.x && p.x <= rect.x + rect.width;
  const y_in = rect.y <= p.y && p.y <= rect.y + rect.height;
  return x_in && y_in;
};

const containedIn = (n1: BackendNode, n2: BackendNode): boolean => {
  if (n1.id.split(".").length != n2.id.split(".").length) return false; // rely on same level collision only
  if (!n2.type?.includes("eval")) return false; // only evals will obstruct

  const w1 = Number(n1.measured?.width?.valueOf() ?? 0);
  const h1 = Number(n1.measured?.height?.valueOf() ?? 0);
  const w2 = Number(n2.style?.width?.valueOf() ?? 0);
  const h2 = Number(n2.style?.height?.valueOf() ?? 0);

  const rect: Rect = {
    x: n2.position.x,
    y: n2.position.y,
    width: w2,
    height: h2,
  };

  const t_l: XYPosition = { x: n1.position.x, y: n1.position.y };
  const b_l: XYPosition = { x: n1.position.x, y: n1.position.y + h1 };
  const t_r: XYPosition = { x: n1.position.x + w1, y: n1.position.y };
  const b_r: XYPosition = { x: n1.position.x + w1, y: n1.position.y + h1 };

  const t_l_in = positionInRect(t_l, rect);
  const b_l_in = positionInRect(b_l, rect);
  const t_r_in = positionInRect(t_r, rect);
  const b_r_in = positionInRect(b_r, rect);

  return t_l_in || b_l_in || t_r_in || b_r_in;
};

const getContainingNodes = (
  node: BackendNode,
  nodes: BackendNode[]
): BackendNode[] => {
  return nodes.filter((n) => containedIn(node, n));
};

export const updateGraph = (graph: Graph, new_graph: Graph): Graph => {
  let nodesMap = new Map(graph.nodes.map((node) => [node.id, node]));

  new_graph.nodes = bottomUpLayout(new_graph.nodes, new_graph.edges);
  for (let node of new_graph.nodes) {
    const existing = nodesMap.get(node.id);
    if (!existing) continue;
    const containingNodes = getContainingNodes(existing, new_graph.nodes);
    if (containingNodes.length === 0) node.position = existing.position;
  }

  return { nodes: [...new_graph.nodes], edges: [...new_graph.edges] };
};
