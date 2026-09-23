import { Handle, Position, type NodeProps } from "@xyflow/react";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Maximize2, Trash2 } from "lucide-react";
import { inHandle, outHandle, type BuilderNode } from "./model";

// Mirrors the shapes used by the real Workflow graph nodes (@/nodes/simple.tsx
// and @/nodes/default.tsx): small squares for ports/const/control-flow leaves,
// a wider card for anything with a body (tasks and subgraph references).
const SQUARE_SHAPE_CLASS: Partial<Record<string, string>> = {
  input: "rounded-b-lg border-double border-sidebar-border border-3",
  output: "rounded-t-lg border-double border-sidebar-border border-3",
  const: "rounded-lg border-double border-sidebar-border border-3",
  ifelse: "rounded-[18px]",
};

const isSquareKind = (kind: string): boolean => kind in SQUARE_SHAPE_CLASS;
const isSubgraphRef = (kind: string): kind is "eval" | "loop" | "map" =>
  kind === "eval" || kind === "loop" || kind === "map";

// Same visual convention as the real graph: input handles along the top
// edge, output handles along the bottom edge, evenly spaced. Deliberately
// left un-positioned (no `relative` ancestor) so the absolute offsets escape
// up to React Flow's own node wrapper and land on the outer node edges,
// rather than the edges of whichever inner Card section they're nested in.
const HandleRow = (props: {
  ports: string[];
  type: "target" | "source";
  position: Position.Top | Position.Bottom;
}) => (
  <div
    className={`absolute left-0 w-full flex justify-around items-center gap-1 ${
      props.position === Position.Top
        ? "top-0 -translate-y-1/2"
        : "bottom-0 translate-y-1/2"
    }`}
  >
    {props.ports.map((port) => (
      <Tooltip key={port}>
        <TooltipTrigger asChild>
          <Handle
            type={props.type}
            position={props.position}
            id={props.type === "target" ? inHandle(port) : outHandle(port)}
            style={{ position: "relative", transform: "none", inset: "auto" }}
            className="!static !size-3"
          />
        </TooltipTrigger>
        <TooltipContent>{port}</TooltipContent>
      </Tooltip>
    ))}
  </div>
);

const DeleteButton = (props: { onDelete?: () => void; className?: string }) => (
  <Tooltip>
    <TooltipTrigger asChild>
      <Button
        size="icon"
        variant="destructive"
        className={`nodrag size-5 ${props.className ?? ""}`}
        onClick={(e) => {
          e.stopPropagation();
          props.onDelete?.();
        }}
      >
        <Trash2 className="size-3" />
      </Button>
    </TooltipTrigger>
    <TooltipContent>Delete node</TooltipContent>
  </Tooltip>
);

export function BuilderNodeView({ data }: NodeProps<BuilderNode>) {
  const subtitle =
    data.kind === "task"
      ? `${data.workerName}.${data.taskName}`
      : isSubgraphRef(data.kind)
        ? data.refWorkflowName
        : data.kind === "const"
          ? data.constValue
          : undefined;

  if (isSquareKind(data.kind)) {
    return (
      <div
        className={`relative w-[80px] h-[80px] drag-handle shadow-sm flex items-center justify-center bg-card ${SQUARE_SHAPE_CLASS[data.kind]}`}
      >
        <HandleRow ports={data.inputs} type="target" position={Position.Top} />
        <HandleRow
          ports={data.outputs}
          type="source"
          position={Position.Bottom}
        />
        <DeleteButton
          onDelete={data.onDelete}
          className="absolute -top-2 -right-2"
        />
        <div className="text-center font-semibold text-xs px-2 whitespace-nowrap overflow-hidden text-ellipsis">
          {data.label}
        </div>
      </div>
    );
  }

  return (
    <Card className="w-[180px] gap-2 drag-handle bg-card">
      <CardHeader className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <CardTitle className="text-sm whitespace-nowrap overflow-hidden text-ellipsis">
            {data.label}
          </CardTitle>
          {subtitle && (
            <div className="text-xs text-muted-foreground truncate">
              {subtitle}
            </div>
          )}
        </div>
        <DeleteButton onDelete={data.onDelete} />
      </CardHeader>
      <CardContent className="h-6">
        <HandleRow ports={data.inputs} type="target" position={Position.Top} />
        <HandleRow
          ports={data.outputs}
          type="source"
          position={Position.Bottom}
        />
      </CardContent>
      {isSubgraphRef(data.kind) && (
        <CardFooter className="justify-end nodrag">
          <Tooltip>
            <TooltipTrigger asChild>
              <span>
                <Button size="sm" variant="secondary" disabled>
                  <Maximize2 /> Expand
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent>
              Expanding a referenced Workflow inline isn&apos;t supported yet -
              open it from the component drawer&apos;s link instead.
            </TooltipContent>
          </Tooltip>
        </CardFooter>
      )}
    </Card>
  );
}
