import { NodeInputs } from "@/data/api_types";

export interface InfoProps {
  type:
    | "Logs"
    | "Errors"
    | "Constant value"
    | "Input"
    | "Output"
    | "Lazy if/else"
    | "Eager if/else";
  content: string;
  workflow_id: string;
  node_location: string;
  started_time?: string;
  finished_time?: string;
  output_names?: string[];
  node_inputs?: NodeInputs[];
  has_error: boolean;
  task_name?: string;
}
export interface HandleProps {
  handles: string[];
  id: string;
  isOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
}
