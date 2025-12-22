import { PyEdge, PyNode } from "@/data/api_types";
import { NestedGraph } from "./NestedGraph";

const rewireMap = (map: string, ng: NestedGraph) => {
  // Each in-edge and out-edge is duplicated for each child of the MAP node.
  const children = ng.childrenOfOpen.get(map) ?? [];
  for (const e of ng.inEdges.get(map) ?? []) {
    for (const child of children) ng.addEdge(e.from_node, child.id, e);
    ng.markEdgeDeleted(e);
  }

  for (const e of ng.outEdges.get(map) ?? []) {
    for (const child of children) ng.addEdge(child.id, e.to_node, e);
    ng.markEdgeDeleted(e);
  }
};

const rewireLoop = (loop: string, ng: NestedGraph) => {
  const children = ng.childrenOfOpen.get(loop) ?? [];
  const outputs = ng.nodes.get(loop)?.outputs;

  for (const e of ng.inEdges.get(loop) ?? []) {
    if (outputs?.includes(e.to_port)) {
      // If the input is also an output then only flow into first inner EVAL.
      ng.addEdge(e.from_node, e.to_node + ".L0", e);
    } else {
      // Otherwise the value stays constant and should flow into all children.
      for (const child of children) ng.addEdge(e.from_node, child.id, e);
    }

    ng.markEdgeDeleted(e);
  }

  for (const e of ng.outEdges.get(loop) ?? []) {
    // The final outputs should escape into parent graph.
    const latest = children.at(-1);
    if (latest) {
      ng.addEdge(latest.id, e.to_node, e);
      ng.markEdgeDeleted(e);
    }
  }
};

const rewireEvals = (ev: string, ng: NestedGraph) => {
  // Replace edges into the expanded EVAL with edges to the input nodes inside the EVAL.
  const children = ng.childrenOfOpen.get(ev) ?? [];
  for (const e of ng.inEdges.get(ev) ?? []) {
    // The body graph should remain flowing into the outer EVAL.
    if (e.to_port === "body") continue;

    const newTarget = children.find(
      (x) => x.function_name === "input" && x.value === e.to_port,
    );
    if (newTarget !== undefined) e.to_node = newTarget.id;
  }

  // Replace edges out of the expended EVAL with edges out of the output node inside the EVAL.
  for (const e of ng.outEdges.get(ev) ?? []) {
    const newSource = children.find((x) => x.function_name === "output");
    if (newSource !== undefined) e.from_node = newSource.id;
  }
};

export const amalgamateGraphData = (
  openNodes: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>,
  openEvals: string[],
  openLoops: string[],
  openMaps: string[],
): {
  nodes: PyNode[];
  edges: PyEdge[];
} => {
  const ng = new NestedGraph(openNodes);

  // Rewire EVALs last because LOOPs and MAPs contain EVALs.
  for (const map of openMaps) rewireMap(map, ng);
  for (const loop of openLoops) rewireLoop(loop, ng);
  for (const ev of openEvals) rewireEvals(ev, ng);

  return ng.getGraph();
};
