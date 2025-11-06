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
  workflowId: string;
  node_location: string;
}
export interface HandleProps {
  handles: string[];
  id: string;
  isOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
}
