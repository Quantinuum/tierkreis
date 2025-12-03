import { InfoProps } from "@/components/types";
import { Edge, type Node } from "@xyflow/react";

export type PyNode = {
  id: string;
  status: "Not started" | "Started" | "Error" | "Finished";
  function_name: string;
  node_location: string;
  node_type: string;
  value?: unknown;
  started_time: string;
  finished_time: string;
};
export type BackendNode = Node<{
  name: string;
  status: "Not started" | "Started" | "Error" | "Finished";
  handles: {
    inputs: string[];
    outputs: string[];
  };
  hidden_handles?: {
    inputs: string[];
    outputs: string[];
  };
  hidden_edges?: Edge[];
  workflowId: string;
  node_location: string;
  id: string;
  title: string;
  label?: string;
  pinned: boolean;
  value: string | null;
  setInfo?: (info: InfoProps) => void;
  is_expanded: boolean;
  isTooltipOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
  started_time: string;
  finished_time: string;
  node_type: "eval" | "loop" | "map";
}>;
export type AppNode = BackendNode;
