import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "./models";
import { loc_peek } from "@/data/loc";
import { getContainingNodes } from "@/nodes/layout";

export const updateGraph = (graph: Graph, new_graph: Graph): Graph => {
  const nodesMap = new Map(graph.nodes.map((node) => [node.id, node]));

  new_graph.nodes = bottomUpLayout(new_graph.nodes, new_graph.edges);
  for (const node of new_graph.nodes) {
    const existing = nodesMap.get(node.id);

    if (!existing) continue;
    // Loop or map nodes always use calculated position.
    if (loc_peek(node.id)?.includes("L")) continue;
    if (loc_peek(node.id)?.includes("M")) continue;

    // If a node has been moved and then obscured by a node expansion
    // then we used the calculated position (unobscured) rather than the moved one.
    const containingNodes = getContainingNodes(existing, new_graph.nodes);
    if (containingNodes.length === 0) node.position = existing.position;
  }

  return { nodes: [...new_graph.nodes], edges: [...new_graph.edges] };
};
