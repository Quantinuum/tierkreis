import type { Edge, EdgeTypes } from "@xyflow/react";
import CustomEdge from "./customEdge";
export const initialEdges = [] as Edge[];
export const edgeTypes = {
  "custom-edge": CustomEdge,
  // Add your custom edge types here!
} satisfies EdgeTypes;
