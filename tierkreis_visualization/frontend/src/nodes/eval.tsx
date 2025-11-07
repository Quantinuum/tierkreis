import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Edge, type NodeProps, useReactFlow } from "@xyflow/react";

import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { NodeStatusIndicator } from "@/components/StatusIndicator";
import { type BackendNode } from "./types";
import { hideChildren } from "./hide_children";
import { Minus, Plus } from "lucide-react";
import { InfoProps } from "@/components/types";

function replaceEval(
  nodeId: string,
  newNodes: BackendNode[],
  newEdges: Edge[],
  oldNodes: BackendNode[],
  oldEdges: Edge[]
) {
  // replaces an eval node with its nested subgraph
  const oldEdgesCopy: Edge[] = JSON.parse(JSON.stringify(oldEdges));
  // we only care about the last part of the id as number
  newNodes.sort(
    (a, b) =>
      Number(a.id.substring(a.id.lastIndexOf(":"), a.id.length)) -
      Number(b.id.substring(b.id.lastIndexOf(":"), b.id.length))
  );
  oldEdgesCopy.forEach((edge) => {
    if (edge.target == nodeId && edge.label != "Graph Body") {
      // find the correct node which has an output handle of the form id:\dport_name
      let found = false;
      for (const node of newNodes) {
        if (node.id.startsWith(nodeId)) {
          node.data.handles.outputs.forEach((value) => {
            if (edge.targetHandle?.endsWith(value)) {
              node.data.handles.inputs.push(value);
              edge.targetHandle = node.id + "_" + value;
              edge.target = node.id;
              found = true;
            }
          });
          if (found) {
            break;
          }
        }
      }
      if (!found) {
        // workaround for elements inside map, only works correctly if the unfolded value is mapped to the first input
        const node = newNodes[0];
        const value = edge.targetHandle?.split("_")[1] || "";
        node.data.handles.inputs.push(value);
        edge.targetHandle = node.id + "_" + value;
        edge.target = node.id;
      }
    }
    if (edge.source == nodeId) {
      console.log(edge.label);
      let found = false;
      for (let index = newNodes.length - 1; index >= 0; index--) {
        const node = newNodes[index];
        if (node.id.startsWith(nodeId)) {
          node.data.handles.inputs.forEach((value) => {
            if (edge.sourceHandle?.endsWith(value) && value != "loop") {
              // != "loop" is a hack to fix loops with a single iteration TODO: check if this really works
              node.data.handles.outputs.push(value);
              edge.sourceHandle = node.id + "_" + value;
              edge.source = node.id;
              found = true;
            }
          });
          if (found) {
            break;
          }
        }
      }
      if (!found) {
        // workaround for elements inside map, only works correctly if there is a single output
        const node = newNodes[newNodes.length - 1];
        const value = edge.sourceHandle?.split("_")[1] || "";
        node.data.handles.outputs.push(value);
        edge.sourceHandle = node.id + "_" + value;
        edge.source = node.id;
      }
    }
  });
  // update the internal state of the eval node
  oldNodes = oldNodes
    .map((node) => {
      if (node.id === nodeId) {
        node.position = { x: 0, y: 0 };
        node.data.hidden_handles = {
          inputs: node.data.handles.inputs,
          outputs: node.data.handles.outputs,
        };
        node.data.hidden_edges = oldEdges.filter(
          (edge) =>
            (edge.target === nodeId || edge.source === nodeId) &&
            edge.label !== "Graph Body"
        );
        console.log(node.data.hidden_edges);
        node.data.handles = { inputs: ["body"], outputs: [] };
        node.data.is_expanded = true;
      }
      return node;
    })
    .filter((node): node is BackendNode => node !== undefined);
  const tmpEdges = oldEdgesCopy.filter(
    (edge) =>
      (edge.target !== nodeId && edge.source !== nodeId) ||
      edge.label === "Graph Body"
  );
  return {
    nodes: [...oldNodes, ...newNodes],
    edges: [...tmpEdges, ...newEdges],
  };
}

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
          hoveredId={node_data.id}
          setHoveredId={node_data.setHoveredId}
        />
        <div className="grid justify-items-end">
          <ZoomOutButton wid={wid} loc={loc} node_loc={node_loc} />
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
  const loadChildren = async (
    workflowId: string,
    node_location: string,
    parentId: string,
    setInfo: ((info: InfoProps) => void) | undefined
  ) => {
    const url = `${URL}/${workflowId}/nodes/${node_location}`;
    fetch(url, { method: "GET", headers: { Accept: "application/json" } })
      .then((response) => response.json())
      .then((data) => {
        const nodes = parseNodes(
          data.nodes,
          data.edges,
          workflowId,
          setInfo,
          parentId
        );
        const edges = parseEdges(data.edges, parentId);
        const oldEdges = reactFlowInstance.getEdges();
        const oldNodes = reactFlowInstance.getNodes();
        const { nodes: newNodes, edges: newEdges } = replaceEval(
          parentId,
          nodes,
          edges,
          oldNodes,
          oldEdges
        );
        const positionedNodes = bottomUpLayout(newNodes, [
          ...newEdges,
          ...oldEdges,
        ]);
        reactFlowInstance.setNodes(positionedNodes);
        reactFlowInstance.setEdges(newEdges);
      });
  };

  return (
    <NodeStatusIndicator status={node_data.status}>
      <Card onDoubleClick={handleDoubleClick} className="w-[180px] gap-2">
        <CardHeader>
          <CardTitle className="overflow-wrap flex-grow">
            {node_data.title}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center">
            {node_data.status != "Not started" && (
              <Button
                className="flex-none"
                variant="secondary"
                size="icon"
                onClick={() =>
                  loadChildren(
                    node_data.workflowId,
                    node_data.node_location,
                    node_data.id,
                    node_data.setInfo
                  )
                }
              >
                <Plus />
              </Button>
            )}
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
