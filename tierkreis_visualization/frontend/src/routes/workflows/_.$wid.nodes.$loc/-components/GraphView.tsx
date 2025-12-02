import Layout from "@/components/layout";
import { InfoProps, Workflow } from "@/components/types";
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
import { Eye, EyeClosed, FolderSync, Network } from "lucide-react";

export const GraphView = (props: {
  nodes: BackendNode[];
  edges: Edge[];
  onNodesChange: OnNodesChange<BackendNode>;
  workflow_id: string;
  workflows: Workflow[];
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
    setAreTooltipsOpen((prev) => !prev);
    reactFlowInstance.getNodes().forEach((node) => {
      reactFlowInstance.updateNodeData(node.id, {
        isTooltipOpen: tooltipsOpen,
      });
    });
  };

  const onNodeDrag: OnNodeDrag = useCallback((_, node) => {
    node.data.pinned = true;
  }, []);

  const ns = props.nodes.sort((a, b) =>
    a.id < b.id ? -1 : a.id > b.id ? 1 : 0
  );

  return (
    <Layout
      workflows={props.workflows}
      workflowId={props.workflow_id}
      info={props.infoProps}
    >
      <ReactFlow<BackendNode, Edge>
        nodes={ns}
        edges={props.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={props.onNodesChange}
        onNodeDrag={onNodeDrag}
        fitView
      >
        <Background />
        <Controls showZoom={false} showInteractive={false}>
          <SidebarTrigger style={{ fill: "none" }} />
          <ControlButton
            onClick={() => {
              reactFlowInstance.setEdges(reactFlowInstance.getEdges());
              reactFlowInstance.setNodes(
                bottomUpLayout(
                  reactFlowInstance.getNodes(),
                  reactFlowInstance.getEdges()
                )
              );
              reactFlowInstance.fitView({ padding: 0.1 });
            }}
          >
            <Network style={{ fill: "none" }} />
          </ControlButton>
          <ControlButton
            onClick={() => {
              localStorage.clear();
            }}
          >
            <FolderSync style={{ fill: "none" }} />
          </ControlButton>
          <ControlButton
            onClick={() => {
              handleToggleTooltips();
            }}
          >
            {true ? (
              <Eye style={{ fill: "none" }} />
            ) : (
              <EyeClosed style={{ fill: "none" }} />
            )}
          </ControlButton>
        </Controls>
      </ReactFlow>
    </Layout>
  );
};
