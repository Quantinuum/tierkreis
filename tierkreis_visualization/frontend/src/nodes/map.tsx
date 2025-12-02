import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Edge, useReactFlow, type NodeProps } from "@xyflow/react";

import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { NodeStatusIndicator } from "@/components/StatusIndicator";
import { Button } from "@/components/ui/button";
import { parseNodes } from "@/graph/parseGraph";
import { type BackendNode } from "@/nodes/types";
import { Plus, Minus } from "lucide-react";
import { bottomUpLayout } from "@/graph/layoutGraph";
import { hideChildren } from "./hide_children";
import { fetchNode } from "@/data/api";
import { closeLink, openLink } from "./links";
import { useNavigate, useParams } from "@tanstack/react-router";

function replaceMap(
  nodeId: string,
  newNodes: BackendNode[],
  oldNodes: BackendNode[],
  oldEdges: Edge[]
) {
  // copy over all the inputs and outputs from the map node to its children
  let edges: Edge[] = JSON.parse(JSON.stringify(oldEdges));
  const edgesToRemove: string[] = [];
  const newEdges: Edge[] = [];
  edges.forEach((edge) => {
    if (edge.target == nodeId) {
      edgesToRemove.push(edge.id);
      newNodes.forEach((node) => {
        if (typeof edge.targetHandle === "string") {
          node.data.handles.inputs.push(
            edge.targetHandle.replace(`${nodeId}_`, "")
          );
        }
        const newEdge = {
          ...edge,
          target: node.id,
          targetHandle:
            node.id + "_" + edge.targetHandle?.replace(`${nodeId}_`, ""),
          id: edge.id.replace(`${nodeId}`, `${node.id}`),
        };
        newEdges.push(newEdge);
      });
    }
    if (edge.source == nodeId) {
      edgesToRemove.push(edge.id);
      newNodes.forEach((node) => {
        if (typeof edge.sourceHandle === "string") {
          node.data.handles.outputs.push(
            edge.sourceHandle.replace(`${nodeId}_`, "")
          );
        }
        const newEdge = {
          ...edge,
          source: node.id,
          sourceHandle:
            node.id + "_" + edge.sourceHandle?.replace(`${nodeId}_`, ""),
          id: edge.id.replace(`${nodeId}`, `${node.id}`),
        };
        newEdges.push(newEdge);
      });
    }
  });
  // remove the body nodes (might not want to do that in the future)
  // update the internal state of the map node
  oldNodes = oldNodes
    .map((node) => {
      if (node.id === nodeId) {
        node.position = { x: 0, y: 0 };
        node.data.hidden_handles = node.data.handles;
        node.data.hidden_edges = edges.filter(
          (edge) => edge.target === nodeId || edge.source === nodeId
        );
        node.data.handles = { inputs: [], outputs: [] };
        node.data.is_expanded = true;
      }
      return node;
    })
    .filter((node): node is BackendNode => node !== undefined);
  edges = edges.filter((edge) => !edgesToRemove.includes(edge.id));
  // there are no edges between the newNodes
  return {
    nodes: [...oldNodes, ...newNodes],
    edges: [...edges, ...newEdges],
  };
}

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
          {closeLink(wid, loc, node_loc)}
        </div>
      </NodeStatusIndicator>
    );
  }
  return (
    <NodeStatusIndicator status={node_data.status}>
      {}
      <Card onDoubleClick={handleDoubleClick} className="w-[180px] gap-2">
        <CardHeader>
          <CardTitle>{node_data.title}</CardTitle>
        </CardHeader>

        <CardContent>
          <div className="flex items-center justify-center">
            {openLink(wid, loc, node_loc)}
          </div>
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
