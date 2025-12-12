export interface InfoProps {
  type: "Logs" | "Errors" | "Constant value" | "Input" | "Output";
  content: string;
}
export interface HandleProps {
  handles: string[];
  id: string;
  isOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
}
