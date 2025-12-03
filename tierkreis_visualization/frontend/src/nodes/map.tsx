import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { type NodeProps } from "@xyflow/react";

import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { NodeStatusIndicator } from "@/components/StatusIndicator";
import { type BackendNode } from "@/nodes/types";
import { zoomOutButton, ZoomInButton } from "./node_navigation";
import { useNavigate, useParams } from "@tanstack/react-router";

export function MapNode({ data: node_data }: NodeProps<BackendNode>) {
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
        <div className="grid justify-items-end">
          {zoomOutButton(wid, loc, node_loc)}
        </div>
      </NodeStatusIndicator>
    );
  }

  const inButton = node_data.status != "Not started" && (
    <ZoomInButton wid={wid} loc={loc} node_loc={node_loc} node_type="map" />
  );

  return (
    <NodeStatusIndicator status={node_data.status}>
      {}
      <Card onDoubleClick={handleDoubleClick} className="w-[180px] gap-2">
        <CardHeader>
          <CardTitle>{node_data.title}</CardTitle>
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
    </NodeStatusIndicator>
  );
}
