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
  // console.log("update graph");
  // console.log(nodes);
  // console.log(edges);
  // console.log(new_graph);

  let nodesMap = new Map<string, BackendNode>();
  if (nodes) {
    nodesMap = new Map(nodes.map((node) => [node.id, node]));
  }

  const newNodes = bottomUpLayout(new_graph.nodes, new_graph.edges);
  const hiddenEdges = new Set<string>();
  for (let newNode of newNodes) {
    const existingNode = nodesMap.get(newNode.id);
    if (!existingNode) continue;
    newNode.position = existingNode.position;
  }

  const edgeIds = new Set(edges.map((edge) => edge.id));
  const newEdges = new_graph.edges.filter((edge) => !edgeIds.has(edge.id));
  //   console.log(hiddenEdges);
  const oldEdges = [...edges, ...newEdges].filter(
    (edge) => !hiddenEdges.has(edge.source) && !hiddenEdges.has(edge.target)
  );
  // console.log("new nodes", Array.from(nodesMap.values()));
  // console.log("old edges", oldEdges);
  return { nodes: [...newNodes], edges: [...oldEdges] };
};
