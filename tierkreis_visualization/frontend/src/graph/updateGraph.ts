import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "./models";
import { loc_children, loc_peek } from "@/data/loc";
import { PyEdge, PyNode } from "@/data/api_types";
import { getContainingNodes } from "@/nodes/layout";

const DELETE_TAG = "fordeletion";

const rewire = (
  e: PyEdge,
  n: PyNode,
  rewire_at: "source" | "target"
): PyEdge => {
  return rewire_at === "source"
    ? { ...e, from_node: n.id }
    : { ...e, to_node: n.id };
};

const rewireAll = (
  ns: PyNode[],
  es: PyEdge[],
  e: PyEdge,
  expanded_loc: string,
  rewire_at: "source" | "target"
) => {
  const newTargets = loc_children(expanded_loc, ns);
  const newEdges = newTargets.map((x) => {
    return rewire(e, x, rewire_at);
  });
  e.from_node = DELETE_TAG;
  es.push(...newEdges);
};

export const amalgamateGraphData = (
  evalData: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>,
  openEvals: string[],
  openLoops: string[],
  openMaps: string[]
): {
  nodes: PyNode[];
  edges: PyEdge[];
} => {
  const ns = [];
  let es: PyEdge[] = [];

  for (const loc in evalData) {
    ns.push(...(evalData[loc]?.nodes ?? []));
    es.push(...(evalData[loc]?.edges ?? []));
  }

  // Collect edges into and out of expanded nodes.
  const inEdges = new Map<string, PyEdge[]>();
  const outEdges = new Map<string, PyEdge[]>();

  for (const e of es) {
    if (Object.keys(evalData).includes(e.from_node)) {
      outEdges.set(e.from_node, [...(outEdges.get(e.from_node) ?? []), e]);
    }
    if (Object.keys(evalData).includes(e.to_node)) {
      inEdges.set(e.to_node, [...(inEdges.get(e.to_node) ?? []), e]);
    }
  }

  // Rewire edges overlapping MAP boundary
  for (const map of openMaps) {
    for (const e of inEdges.get(map) ?? []) rewireAll(ns, es, e, map, "target");
    for (const e of outEdges.get(map) ?? [])
      rewireAll(ns, es, e, map, "source");
  }

  // Rewire edges overlapping LOOP boundary
  for (const loop of openLoops) {
    const outputs = ns.find((x) => x.id === loop)?.outputs;
    for (const e of inEdges.get(loop) ?? []) {
      if (outputs?.includes(e.to_port)) e.to_node = e.to_node + ".L0";
      else rewireAll(ns, es, e, loop, "target");
    }

    for (const e of outEdges.get(loop) ?? []) {
      const latest = loc_children(e.from_node, ns).at(-1);
      if (latest) e.from_node = latest.id;
    }
  }

  // Rewire inputs of open EVALs
  for (const e of es) {
    if (!openEvals.includes(e.to_node)) continue;
    if (e.to_port === "body") continue;

    const newTarget = evalData[e.to_node]?.nodes.find(
      (x) => x.function_name === "input" && x.value === e.to_port
    );
    if (newTarget !== undefined) e.to_node = newTarget.id;
  }

  // Rewire outputs of open EVALs
  for (const e of es) {
    if (!openEvals.includes(e.from_node)) continue;

    const newSource = evalData[e.from_node]?.nodes.find(
      (x) => x.function_name === "output"
    );
    if (newSource !== undefined) e.from_node = newSource.id;
  }

  es = es.filter((x) => x.to_node !== DELETE_TAG && x.from_node !== DELETE_TAG);

  return { nodes: ns, edges: es };
};

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
