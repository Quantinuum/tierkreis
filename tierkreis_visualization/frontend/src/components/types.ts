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
}
export interface HandleProps {
  handles: string[];
  id: string;
  isOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
}
