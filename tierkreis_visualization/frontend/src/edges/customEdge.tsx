import {
  BaseEdge,
  EdgeLabelRenderer,
  EdgeProps,
  getBezierPath,
} from "@xyflow/react";
import { Braces, GitGraph } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";

function LabelButton({
  children,
  labelX,
  labelY,
}: {
  children: React.ReactNode;
  labelX: number;
  labelY: number;
}) {
  return (
    <Button
      variant={"outline"}
      size="sm"
      color="gray"
      className="text-lg"
      style={{
        position: "absolute",
        transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
      }}
    >
      {children}
    </Button>
  );
}

export default function CustomEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  label,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
  });

  let content;
  if (label === undefined || label === null || label === "null") {
    return <BaseEdge id={id} path={edgePath} />;
  } else if (label === "Graph Body") {
    content = (
      <LabelButton labelX={labelX} labelY={labelY}>
        <GitGraph />
      </LabelButton>
    );
  } else if (typeof label == "string" && label.length < 5) {
    content = (
      <LabelButton labelX={labelX} labelY={labelY}>
        {label}
      </LabelButton>
    );
  } else {
    content = (
      <LabelButton labelX={labelX} labelY={labelY}>
        <HoverCard>
          <HoverCardTrigger style={{ pointerEvents: "all" }} asChild>
            <Braces />
          </HoverCardTrigger>
          <HoverCardContent
            className="overflow-auto"
            style={{ maxHeight: "50vh", maxWidth: "100vw", minWidth: "25vw" }}
          >
            <pre>{label}</pre>
          </HoverCardContent>
        </HoverCard>
      </LabelButton>
    );
  }

  return (
    <>
      <BaseEdge id={id} path={edgePath} />
      <EdgeLabelRenderer>
        <>{content}</>
      </EdgeLabelRenderer>
    </>
  );
}
