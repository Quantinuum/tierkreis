import { BackendNode } from "@/nodes/types";
import { bottomUpLayout } from "./layoutGraph";
import { Edge } from "@xyflow/react";

export const updateGraph = (
  graph: { nodes: BackendNode[]; edges: Edge[] },
  new_graph: {
    nodes: BackendNode[];
    edges: Edge[];
  }
): { nodes: BackendNode[]; edges: Edge[] } => {
  let nodes = graph.nodes;
  let edges = graph.edges;

  if (nodes.length !== new_graph.nodes.length) {
    new_graph.nodes = bottomUpLayout(new_graph.nodes, new_graph.edges);
    return new_graph;
  }

  let nodesMap = new Map(new_graph.nodes.map((node) => [node.id, node]));

  const hiddenEdges = new Set<string>();
  for (let node of nodes) {
    const updatedNode = nodesMap.get(node.id);
    if (!updatedNode) continue;

    node.data.status = updatedNode.data.status;
  }

  const edgeIds = new Set(edges.map((edge) => edge.id));
  const newEdges = new_graph.edges.filter((edge) => !edgeIds.has(edge.id));
  const oldEdges = [...edges, ...newEdges].filter(
    (edge) => !hiddenEdges.has(edge.source) && !hiddenEdges.has(edge.target)
  );
  return { nodes: [...nodes], edges: [...edges] };
};
