import { BackendNode, PyNode } from "@/nodes/types";
import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "@/routes/workflows/_.$wid.nodes.$loc/-components/models";
import { Rect, XYPosition } from "@xyflow/react";
import { PyEdge } from "@/edges/types";
import { loc_depth, loc_peek } from "@/data/loc";

const positionInRect = (p: XYPosition, rect: Rect): boolean => {
  const x_in = rect.x <= p.x && p.x <= rect.x + rect.width;
  const y_in = rect.y <= p.y && p.y <= rect.y + rect.height;
  return x_in && y_in;
};

const containedIn = (n1: BackendNode, n2: BackendNode): boolean => {
  if (loc_depth(n1.id) != loc_depth(n2.id)) return false; // rely on same level collision only
  if (
    !(
      n2.type?.includes("eval") ||
      n2.type?.includes("loop") ||
      n2.type?.includes("map")
    )
  )
    return false; // only nested nodes will obstruct

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

    const prefix = e.to_node + ".M";
    const current_depth = loc_depth(e.to_node);
    const newTargets = ns.filter(
      (x) => x.id.startsWith(prefix) && loc_depth(x.id) == current_depth + 1
    );
    const newEdges = newTargets.map((x) => {
      return { ...e, to_node: x.id };
    });
    es = [...es, ...newEdges];
  }

  // Rewire outputs of open MAPs
  for (let e of es) {
    if (!openMaps.includes(e.from_node)) continue;

    const prefix = e.from_node + ".M";
    const current_depth = loc_depth(e.from_node);
    const newSources = ns.filter(
      (x) => x.id.startsWith(prefix) && loc_depth(x.id) == current_depth + 1
    );
    const newEdges = newSources.map((x) => {
      return { ...e, from_node: x.id };
    });
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

  // TODO: rewire outputs of open EVALs
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
