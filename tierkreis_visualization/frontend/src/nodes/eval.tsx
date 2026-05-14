import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { NodeResizeControl, type NodeProps } from "@xyflow/react";

import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { NodeStatusIndicator } from "@/components/StatusIndicator";
import { type BackendNode } from "./types";
import { useNavigate, useParams } from "@tanstack/react-router";
import { ZoomOutButton, ZoomInButton } from "./node_navigation";
import { bg_color } from "@/components/colors";
import { Button } from "@/components/ui/button";
import { Move, Scaling } from "lucide-react";

export function EvalNode({ data: node_data }: NodeProps<BackendNode>) {
  const navigate = useNavigate();
  const wid = node_data.workflowId;
  const node_loc = node_data.node_location;
  let { loc } = useParams({ strict: false });
  loc = loc ?? "-";
  const handleDoubleClick = () => {
    navigate({
      to: "/workflows/$wid/nodes/$loc",
      params: { wid, loc: node_loc },
    });
  };

  if (node_data.is_expanded) {
    return (
      <NodeStatusIndicator status={node_data.status}>
        <InputHandleArray
          handles={node_data.handles.inputs}
          id={node_data.id}
          isOpen={node_data.isTooltipOpen}
          hoveredId={node_data.hoveredId}
          setHoveredId={node_data.setHoveredId}
        />
        <div className="flex justify-end">
          <div className="drag-handle">
            <Button className="flex-none m-1" variant="secondary" size="icon">
              <Move />
            </Button>
          </div>
          <ZoomOutButton wid={wid} loc={loc} node_loc={node_loc} />
          <div>
            <Button className="flex-none m-1" variant="secondary" size="icon">
              <Scaling />
              <NodeResizeControl
                className="flex-none m-1"
                autoScale={false}
                style={{
                  // Hack the position to where we want it, anyone smartet than me feel free to fix this
                  background: "transparent",
                  border: "none",
                  position: "absolute",
                  left: "97%",
                  top: "18px",
                  width: "36px",
                  height: "36px",
                }}
                minWidth={100}
                minHeight={50}
                position="top-right"
              ></NodeResizeControl>
            </Button>
          </div>
        </div>
        <OutputHandleArray
          handles={node_data.handles.outputs}
          id={node_data.id}
          isOpen={node_data.isTooltipOpen}
          hoveredId={node_data.hoveredId}
          setHoveredId={node_data.setHoveredId}
        />
      </NodeStatusIndicator>
    );
  }

  const inButton = (
    <ZoomInButton
      wid={wid}
      loc={loc}
      node_loc={node_loc}
      node_type={node_data.node_type}
    />
  );

  return (
    <Card
      onDoubleClick={handleDoubleClick}
      className={"w-[180px] gap-2 drag-handle " + bg_color(node_data.status)}
    >
      <CardHeader>
        <CardTitle className="overflow-wrap flex-grow">
          {node_data.title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-center">{inButton}</div>
        <InputHandleArray
          handles={node_data.handles.inputs}
          id={node_data.id}
          isOpen={node_data.isTooltipOpen}
          hoveredId={node_data.hoveredId}
          setHoveredId={node_data.setHoveredId}
        />
        <OutputHandleArray
          handles={node_data.handles.outputs}
          id={node_data.id}
          isOpen={node_data.isTooltipOpen}
          hoveredId={node_data.hoveredId}
          setHoveredId={node_data.setHoveredId}
        />
      </CardContent>
      <CardFooter
        className="flex justify-content justify-start"
        style={{ padding: "-5px" }}
      ></CardFooter>
    </Card>
  );
}
