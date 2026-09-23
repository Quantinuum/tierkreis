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
import { Clock, Eye, FolderSync, Network, Scan, Workflow } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { NodeSearch } from "@/components/NodeSearch";
import { Timeline } from "./Timeline";
import { TraceWaterfall } from "@/routes/monitor/-components/TraceWaterfall";

export const GraphView = (props: {
  nodes: BackendNode[];
  edges: Edge[];
  onNodesChange: OnNodesChange<BackendNode>;
  workflow_id: string;
  loc: string;
  workflows: WorkflowDisplay[];
  infoProps: InfoProps;
  setInfo: (arg: InfoProps) => void;
  initialView?: "graph" | "timeline";
}) => {
  const reactFlowInstance = useReactFlow<BackendNode, Edge>();
  const [tooltipsOpen, setAreTooltipsOpen] = useState(false);
  const [hoveredId, setHoveredId] = useState<string>("");
  const [view, setView] = useState<"graph" | "timeline">(
    props.initialView ?? "graph",
  );

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
      <div className="relative flex-1">
        <div className="absolute top-2 left-2 z-20 flex items-start gap-2">
          {view === "graph" && <NodeSearch nodes={props.nodes} />}
        </div>
        <div className="absolute top-2 right-2 z-20 flex gap-1">
          <Button
            size="sm"
            variant={view === "graph" ? "default" : "outline"}
            onClick={() => setView("graph")}
          >
            <Workflow /> Graph
          </Button>
          <Button
            size="sm"
            variant={view === "timeline" ? "default" : "outline"}
            onClick={() => setView("timeline")}
          >
            <Clock /> Timeline
          </Button>
        </div>
        {view === "timeline" ? (
          <div className="h-full overflow-y-auto flex flex-col divide-y">
            <div className="flex-1 min-h-0">
              <Timeline nodes={props.nodes} />
            </div>
            <div className="p-6">
              <div className="text-lg font-semibold mb-2">
                Full run trace (attempt 0)
              </div>
              <TraceWaterfall runId={props.workflow_id} attempt={0} />
            </div>
          </div>
        ) : (
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
            <Controls
              showZoom={false}
              showInteractive={false}
              showFitView={false}
            >
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
        )}
      </div>
    </Layout>
  );
};
