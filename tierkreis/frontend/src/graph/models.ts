import { BackendNode } from "@/nodes/types";
import { Edge } from "@xyflow/react";

export type Graph = {
  nodes: BackendNode[];
  edges: Edge[];
};
