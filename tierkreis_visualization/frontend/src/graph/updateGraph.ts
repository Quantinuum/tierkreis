import { BackendNode } from "@/nodes/types";
import { bottomUpLayout } from "./layoutGraph";
import { Edge } from "@xyflow/react";
import { Graph } from "@/routes/workflows/_.$wid.nodes.$loc/-components/models";

const nodesIntersect = (n1: BackendNode, n2: BackendNode): boolean => {
  if (n1.id.split(".").length != n2.id.split(".").length) return false; // rely on same level collision only
  if (!n2.type?.includes("eval")) return false; // only evals will obstruct

  const x_intersects =
    n2.position.x <= n1.position.x &&
    n1.position.x <= n2.position.x + (n2.style?.width ?? 0);

  const y_intersects =
    n2.position.y <= n1.position.y &&
    n1.position.y <= n2.position.y + (n2.style?.height ?? 0);
  console.log(y_intersects);

  return x_intersects && y_intersects;
};

const getContainingNodes = (
  node: BackendNode,
  nodes: BackendNode[]
): BackendNode[] => {
  let intersection = [];
  for (let n of nodes) {
    if (n.id === node.id) continue;
    if (nodesIntersect(node, n)) intersection.push(n);
  }
  return intersection;
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
