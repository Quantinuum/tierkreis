import type { NodeTypes } from "@xyflow/react";

import { DefaultNode } from "./default";
import { EvalNode } from "./eval";
import { MapNode } from "./map";
import { AppNode } from "./types";
import { SimpleNode } from "./simple";
import { TaskNode } from "./task";

export const initialNodes = [] as AppNode[];

export const nodeTypes = {
  "eval-node": EvalNode,
  "map-node": MapNode,
  "default-node": DefaultNode,
  "simple-node": SimpleNode,
  "task-node": TaskNode,
} satisfies NodeTypes;
