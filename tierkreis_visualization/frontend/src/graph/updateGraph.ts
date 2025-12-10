import { bottomUpLayout } from "./layoutGraph";
import { Graph } from "./models";
import { loc_children, loc_peek } from "@/data/loc";
import { PyEdge, PyNode } from "@/data/api_types";
import { getContainingNodes } from "@/nodes/layout";

const DELETE_TAG = "fordeletion";

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
  let es = [];
  console.log(evalData);

  for (const loc in evalData) {
    ns.push(...(evalData[loc]?.nodes ?? []));
    es.push(...(evalData[loc]?.edges ?? []));
  }
  console.log([...ns]);
  console.log([...es]);

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
  console.log(inEdges);
  console.log(outEdges);

  // Rewire inputs of open MAPs
  for (const e of es) {
    if (!openMaps.includes(e.to_node)) continue;

    const newTargets = loc_children(e.to_node, ns);
    const newEdges = newTargets.map((x) => {
      return { ...e, to_node: x.id };
    });
    e.to_node = DELETE_TAG;
    es = [...es, ...newEdges];
  }

  // Rewire outputs of open MAPs
  for (const e of es) {
    if (!openMaps.includes(e.from_node)) continue;

    const newSources = loc_children(e.from_node, ns);
    const newEdges = newSources.map((x) => {
      return { ...e, from_node: x.id };
    });
    e.from_node = DELETE_TAG;
    es = [...es, ...newEdges];
  }

  for (const loop of openLoops) {
    const loopNode = ns.find((x) => x.id === loop);
    const outputs = loopNode?.outputs;
    for (const e of inEdges.get(loop) ?? []) {
      if (outputs?.includes(e.to_port)) e.to_node = e.to_node + ".L0";
      else {
        const newTargets = loc_children(e.to_node, ns);
        const newEdges = newTargets.map((x) => {
          return { ...e, to_node: x.id };
        });
        e.from_node = DELETE_TAG;
        es = [...es, ...newEdges];
      }
    }

    for (const e of outEdges.get(loop) ?? []) {
      console.log(evalData[loop]);

      const newSources = loc_children(e.from_node, ns);
      const latest = newSources.at(-1);

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
