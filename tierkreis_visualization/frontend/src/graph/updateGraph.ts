import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "./models";
import { loc_children, loc_depth, loc_peek } from "@/data/loc";
import { PyEdge, PyNode } from "@/data/api_types";
import { getContainingNodes } from "@/nodes/layout";

export const amalgamateGraphData = (
  evalData: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>,
  openEvals: string[],
  openLoops: string[],
  openMaps: string[]
): {
  nodes: PyNode[];
  edges: PyEdge[];
} => {
  let ns = [];
  let es = [];

  for (let loc in evalData) {
    ns.push(...evalData[loc].nodes);
    es.push(...evalData[loc].edges);
  }

  // Rewire inputs of open MAPs
  for (let e of es) {
    if (!openMaps.includes(e.to_node)) continue;

    const newTargets = loc_children(
      e.to_node,
      ns.map((x) => x.id)
    );
    const newEdges = newTargets.map((x) => {
      return { ...e, to_node: x };
    });
    e.to_node = "dummy";
    es = [...es, ...newEdges];
  }

  // Rewire outputs of open MAPs
  for (let e of es) {
    if (!openMaps.includes(e.from_node)) continue;

    const newSources = loc_children(
      e.from_node,
      ns.map((x) => x.id)
    );
    const newEdges = newSources.map((x) => {
      return { ...e, from_node: x };
    });
    e.from_node = "dummy";
    es = [...es, ...newEdges];
  }

  // Rewire inputs of open LOOPs
  for (let e of es) {
    if (!openLoops.includes(e.to_node)) continue;

    const newTargets = loc_children(
      e.to_node,
      ns.map((x) => x.id)
    );
    const newEdges = newTargets.map((x) => {
      return { ...e, to_node: x };
    });
    e.to_node = "dummy";
    es = [...es, ...newEdges];
  }

  // Rewire outputs of open LOOPs
  for (let e of es) {
    if (!openLoops.includes(e.from_node)) continue;

    const newSources = loc_children(
      e.from_node,
      ns.map((x) => x.id)
    );
    const newEdges = newSources.map((x) => {
      return { ...e, from_node: x };
    });
    e.from_node = "dummy";
    es = [...es, ...newEdges];
  }

  // Rewire inputs of open EVALs
  for (let e of es) {
    if (!openEvals.includes(e.to_node)) continue;
    if (e.to_port === "body") continue;

    const newTarget = evalData[e.to_node].nodes.find(
      (x) => x.function_name === "input" && x.value === e.to_port
    );
    if (newTarget !== undefined) e.to_node = newTarget.id;
  }

  // Rewire outputs of open EVALs
  for (let e of es) {
    if (!openEvals.includes(e.from_node)) continue;

    const newSource = evalData[e.from_node].nodes.find(
      (x) => x.function_name === "output"
    );
    if (newSource !== undefined) e.from_node = newSource.id;
  }

  return { nodes: ns, edges: es };
};

export const updateGraph = (graph: Graph, new_graph: Graph): Graph => {
  let nodesMap = new Map(graph.nodes.map((node) => [node.id, node]));

  new_graph.nodes = bottomUpLayout(new_graph.nodes, new_graph.edges);
  for (let node of new_graph.nodes) {
    const existing = nodesMap.get(node.id);

    if (!existing) continue;
    // Loop or map nodes need to be put back in the right place.
    if (loc_peek(node.id)?.includes("L")) continue;
    if (loc_peek(node.id)?.includes("M")) continue;

    const containingNodes = getContainingNodes(existing, new_graph.nodes);
    if (containingNodes.length === 0) node.position = existing.position;
  }

  return { nodes: [...new_graph.nodes], edges: [...new_graph.edges] };
};
