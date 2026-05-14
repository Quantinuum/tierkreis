import Layout from "@/components/layout";
import { InfoProps } from "@/components/types";
import { WorkflowDisplay } from "@/data/api_types";
import { BackendNode } from "@/nodes/types";
import {
  Background,
  ControlButton,
  Controls,
  Edge,
  OnNodeDrag,
  OnNodesChange,
  ReactFlow,
  useReactFlow,
} from "@xyflow/react";
import { useCallback, useState } from "react";
import { nodeTypes } from "@/nodes";
import { edgeTypes } from "@/edges";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { bottomUpLayout } from "@/graph/layoutGraph";
import { Eye, FolderSync, Network, Scan } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export const GraphView = (props: {
  nodes: BackendNode[];
  edges: Edge[];
  onNodesChange: OnNodesChange<BackendNode>;
  workflow_id: string;
  loc: string;
  workflows: WorkflowDisplay[];
  infoProps: InfoProps;
  setInfo: (arg: InfoProps) => void;
}) => {
  const reactFlowInstance = useReactFlow<BackendNode, Edge>();
  const [tooltipsOpen, setAreTooltipsOpen] = useState(false);
  const [hoveredId, setHoveredId] = useState<string>("");

  props.nodes.map((node) => {
    node.data.setInfo = props.setInfo;
    node.data.isTooltipOpen = tooltipsOpen;
    node.data.hoveredId = hoveredId;
    node.data.setHoveredId = (id) => {
      reactFlowInstance.updateNodeData(node.id, {
        hoveredId: id,
      });
      setHoveredId(id);
    };
  });
  const handleToggleTooltips = () => {
    const prev = tooltipsOpen;
    setAreTooltipsOpen((prev) => !prev);
    reactFlowInstance.getNodes().forEach((node) => {
      reactFlowInstance.updateNodeData(node.id, { isTooltipOpen: !prev });
    });
  };

  const onNodeDrag: OnNodeDrag = useCallback((_, node) => {
    node.data.pinned = true;
  }, []);

  const ns = props.nodes.sort((a, b) =>
    a.id < b.id ? -1 : a.id > b.id ? 1 : 0,
  );
  const revertPositionChanges = () => {
    reactFlowInstance.setEdges(reactFlowInstance.getEdges());
    reactFlowInstance.setNodes(
      bottomUpLayout(
        reactFlowInstance.getNodes(),
        reactFlowInstance.getEdges(),
      ),
    );
    reactFlowInstance.fitView({ padding: 0.1 });
  };

  return (
    <Layout
      workflows={props.workflows}
      workflowId={props.workflow_id}
      info={props.infoProps}
      loc={props.loc}
    >
      <ReactFlow<BackendNode, Edge>
        nodes={ns}
        edges={props.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={props.onNodesChange}
        onNodeDrag={onNodeDrag}
        minZoom={0.01}
        defaultEdgeOptions={{ zIndex: -2 }}
        fitView
      >
        <Background />
        <Controls showZoom={false} showInteractive={false} showFitView={false}>
          <Tooltip>
            <TooltipTrigger asChild>
              <ControlButton onClick={() => reactFlowInstance.fitView()}>
                <Scan style={{ fill: "none" }} />
              </ControlButton>
            </TooltipTrigger>
            <TooltipContent>Recenter</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <SidebarTrigger style={{ fill: "none" }} />
            </TooltipTrigger>
            <TooltipContent>Show the sidebar</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <ControlButton onClick={revertPositionChanges}>
                <Network style={{ fill: "none" }} />
              </ControlButton>
            </TooltipTrigger>
            <TooltipContent>Recalculate layout</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <ControlButton onClick={() => localStorage.clear()}>
                <FolderSync style={{ fill: "none" }} />
              </ControlButton>
            </TooltipTrigger>
            <TooltipContent>Clear local storage</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <ControlButton onClick={() => handleToggleTooltips()}>
                <Eye style={{ fill: "none" }} />
              </ControlButton>
            </TooltipTrigger>
            <TooltipContent>Toggle node port names</TooltipContent>
          </Tooltip>
        </Controls>
      </ReactFlow>
    </Layout>
  );
};
