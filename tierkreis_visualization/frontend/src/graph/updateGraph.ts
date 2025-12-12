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

const concatData = (
  evalData: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>
): [PyNode[], PyEdge[]] => {
  const ns: PyNode[] = [];
  const es: PyEdge[] = [];

  for (const loc in evalData) {
    ns.push(...(evalData[loc]?.nodes ?? []));
    es.push(...(evalData[loc]?.edges ?? []));
  }
  return [ns, es];
};

const indexEdges = (
  evalData: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>,
  es: PyEdge[]
): [Map<string, PyEdge[]>, Map<string, PyEdge[]>] => {
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
  return [inEdges, outEdges];
};

const rewireMap = (
  map: string,
  inEdges: Map<string, PyEdge[]>,
  outEdges: Map<string, PyEdge[]>,
  ns: PyNode[],
  es: PyEdge[]
) => {
  for (const e of inEdges.get(map) ?? []) rewireAll(ns, es, e, map, "target");
  for (const e of outEdges.get(map) ?? []) rewireAll(ns, es, e, map, "source");
};

const rewireLoop = (
  loop: string,
  inEdges: Map<string, PyEdge[]>,
  outEdges: Map<string, PyEdge[]>,
  ns: PyNode[],
  es: PyEdge[]
) => {
  const outputs = ns.find((x) => x.id === loop)?.outputs;
  for (const e of inEdges.get(loop) ?? []) {
    if (outputs?.includes(e.to_port)) e.to_node = e.to_node + ".L0";
    else rewireAll(ns, es, e, loop, "target");
  }

  for (const e of outEdges.get(loop) ?? []) {
    const latest = loc_children(e.from_node, ns).at(-1);
    if (latest) e.from_node = latest.id;
  }
};

const rewireEvals = (
  ev: string,
  inEdges: Map<string, PyEdge[]>,
  outEdges: Map<string, PyEdge[]>,
  evalData: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>
) => {
  for (const e of inEdges.get(ev) ?? []) {
    if (e.to_port === "body") continue;

    const newTarget = evalData[e.to_node]?.nodes.find(
      (x) => x.function_name === "input" && x.value === e.to_port
    );
    if (newTarget !== undefined) e.to_node = newTarget.id;
  }

  for (const e of outEdges.get(ev) ?? []) {
    const newSource = evalData[e.from_node]?.nodes.find(
      (x) => x.function_name === "output"
    );
    if (newSource !== undefined) e.from_node = newSource.id;
  }
};

const cleanOrphans = (es: PyEdge[]): PyEdge[] => {
  return es.filter((x) => x.from_node !== DELETE_TAG);
};

export const amalgamateGraphData = (
  openNodes: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>,
  openEvals: string[],
  openLoops: string[],
  openMaps: string[]
): {
  nodes: PyNode[];
  edges: PyEdge[];
} => {
  const [ns, es] = concatData(openNodes);
  const [inEdges, outEdges] = indexEdges(openNodes, es);

  for (const map of openMaps) rewireMap(map, inEdges, outEdges, ns, es);
  for (const loop of openLoops) rewireLoop(loop, inEdges, outEdges, ns, es);
  for (const ev of openEvals) rewireEvals(ev, inEdges, outEdges, openNodes);

  return { nodes: ns, edges: cleanOrphans(es) };
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
