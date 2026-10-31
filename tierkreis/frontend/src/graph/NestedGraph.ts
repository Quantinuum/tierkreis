import { PyEdge, PyGraph, PyNode } from "@/data/api_types";

export class NestedGraph {
  nodes: Map<string, PyNode>;
  childrenOfOpen: Map<string, PyNode[]>;
  inEdges: Map<string, PyEdge[]>;
  outEdges: Map<string, PyEdge[]>;
  DELETE_TAG = "fordeletion";

  constructor(openNodes: Record<string, { nodes: PyNode[]; edges: PyEdge[] }>) {
    this.nodes = new Map<string, PyNode>();
    this.childrenOfOpen = new Map(
      Object.entries(openNodes).map(([k, x]) => [k, x.nodes]),
    );
    this.inEdges = new Map<string, PyEdge[]>();
    this.outEdges = new Map<string, PyEdge[]>();

    for (const { nodes, edges } of Object.values(openNodes)) {
      for (const n of nodes) this.addNode(n);
      for (const e of edges) this.addEdge(e.from_node, e.to_node, e);
    }
  }

  addNode(n: PyNode) {
    this.nodes.set(n.id, n);
  }

  addEdge(from_node: string, to_node: string, model: PyEdge) {
    const e = { ...model, from_node, to_node };
    const newOuts = [...(this.outEdges.get(from_node) ?? []), e];
    this.outEdges.set(from_node, newOuts);

    const newIns = [...(this.inEdges.get(to_node) ?? []), e];
    this.inEdges.set(to_node, newIns);
  }

  markEdgeDeleted(e: PyEdge) {
    e.from_node = this.DELETE_TAG;
  }

  getGraph(): PyGraph {
    let es = Array.from(this.inEdges.values()).flat();
    es = es.filter((x) => x.from_node !== this.DELETE_TAG);
    return { nodes: Array.from(this.nodes.values()), edges: es };
  }
}
