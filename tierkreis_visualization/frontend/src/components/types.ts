export interface InfoProps {
  type: "Logs" | "Errors";
  content: string;
}
export interface Workflow {
  id: string;
  name: string | null;
  start_time: string;
}

export interface HandleProps {
  handles: string[];
  id: string;
  isOpen: boolean;
  hoveredId: string;
  setHoveredId: (id: string) => void;
}
