import { BackendNode } from "@/nodes/types";
import { bottomUpLayout } from "./layoutGraph";
import { Edge } from "@xyflow/react";

export const updateGraph = (
  nodes: BackendNode[],
  edges: Edge[],
  graph: {
    nodes: BackendNode[];
    edges: Edge[];
  }
): { nodes: BackendNode[]; edges: Edge[] } => {
  let nodesMap = new Map();
  if (nodes) {
    nodesMap = new Map(nodes.map((node) => [node.id, node]));
  }
  const newNodes = bottomUpLayout(graph.nodes, graph.edges);
  const hiddenEdges = new Set<string>();
  newNodes.forEach((node) => {
    const existingNode = nodesMap.get(node.id);
    if (existingNode) {
      if (existingNode.type === "group") {
        hiddenEdges.add(existingNode.id);
        return;
      }
      existingNode.data = {
        ...existingNode.data,
        status: node.data.status,
      };
      existingNode.position = {
        ...(nodes.find((n) => n.id === node.id)?.position ?? node.position),
      };
    } else {
      nodesMap.set(node.id, node);
    }
  });

  const edgeIds = new Set(edges.map((edge) => edge.id));
  const newEdges = graph.edges.filter((edge) => !edgeIds.has(edge.id));
  //   console.log(hiddenEdges);
  const oldEdges = [...edges, ...newEdges].filter(
    (edge) => !hiddenEdges.has(edge.source) && !hiddenEdges.has(edge.target)
  );
  //   console.log("new nodes", Array.from(nodesMap.values()));
  //   console.log("old edges", oldEdges);
  return { nodes: Array.from(nodesMap.values()), edges: [...oldEdges] };
};
