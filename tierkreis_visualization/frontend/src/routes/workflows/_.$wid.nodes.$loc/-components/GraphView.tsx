import {
  applyEdgeChanges,
  applyNodeChanges,
  Edge,
  OnEdgesChange,
  OnNodesChange,
  ReactFlow,
  useReactFlow,
  OnNodeDrag,
  addEdge,
} from "@xyflow/react";

import Layout from "@/components/layout";
import { InfoProps, Workflow } from "@/components/types";
import { parseGraph } from "@/graph/parseGraph";
import { Background, ControlButton, Controls } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Eye, EyeClosed, FolderSync, Network } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";

import { SidebarTrigger } from "@/components/ui/sidebar";
import { edgeTypes } from "@/edges";
import { bottomUpLayout } from "@/graph/layoutGraph";
import { nodeTypes } from "@/nodes";
import { BackendNode } from "../../../../nodes/types";
import {
  $api,
  listWorkflowsQuery,
  logsQuery,
  nodeQuery,
} from "../../../../data/api";
import { updateGraph } from "@/graph/updateGraph";

const saveGraph = ({
  key,
  ...graph
}: {
  key: string;
  nodes: BackendNode[];
  edges: Edge[];
  start_time: string;
}) => {
  try {
    localStorage.setItem(key, JSON.stringify(graph));
  } catch {
    return null;
  }
};

const deleteGraph = ({ key }: { key: string }) => {
  try {
    localStorage.removeItem(key);
  } catch {
    return null;
  }
};

const loadGraph = (props: {
  key: string;
}): { nodes: BackendNode[]; edges: Edge[]; start_time: string } | null => {
  try {
    const item = localStorage.getItem(props.key);
    if (item !== null)
      return JSON.parse(item) as {
        nodes: BackendNode[];
        edges: Edge[];
        start_time: string;
      };
    return null;
  } catch {
    return null;
  }
};

const Main = (props: {
  nodes: BackendNode[];
  edges: Edge[];
  onNodesChange: OnNodesChange<BackendNode>;
  onEdgesChange: OnEdgesChange;
  workflow_id: string;
  workflow_start: string;
  workflows: Workflow[];
  infoProps: InfoProps;
  setInfo: (arg: InfoProps) => void;
}) => {
  // Client node state (not definition)
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
  React.useEffect(() => {
    saveGraph({
      key: props.workflow_id,
      nodes: reactFlowInstance.getNodes(),
      edges: reactFlowInstance.getEdges(),
      start_time: props.workflow_start,
    });
  }, [reactFlowInstance, props.workflow_id, props.workflow_start]);

  const onNodeDrag: OnNodeDrag = useCallback((_, node) => {
    node.data.pinned = true;
  }, []);

  return (
    <Layout
      workflows={props.workflows}
      workflowId={props.workflow_id}
      info={props.infoProps}
    >
      <ReactFlow<BackendNode, Edge>
        nodes={props.nodes}
        edges={props.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onEdgesChange={props.onEdgesChange}
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
          // onClick={() => {
          //   handleToggleTooltips();
          // }}
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

type G = {
  nodes: BackendNode[];
  edges: Edge[];
};

export default function GraphView(props: {
  workflow_id: string;
  node_location_str: string;
}) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;
  const [nodes, setNodes] = useState<BackendNode[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);

  const onNodesChange = useCallback(
    (changes) =>
      setNodes((nodesSnapshot) => applyNodeChanges(changes, nodesSnapshot)),
    []
  );
  const onEdgesChange = useCallback(
    (changes) =>
      setEdges((edgesSnapshot) => applyEdgeChanges(changes, edgesSnapshot)),
    []
  );

  const workflowsQuery = listWorkflowsQuery();
  const logs = logsQuery(workflow_id);

  const [info, setInfo] = useState<InfoProps>({
    type: "Logs",
    content: logs.data as string,
  });

  useEffect(() => {
    const url = `/api/workflows/${props.workflow_id}/nodes/${node_location_str}`;
    const ws = new WebSocket(url);
    ws.onmessage = (event) => {
      const newG = parseGraph(JSON.parse(event.data), props.workflow_id);
      setNodes((ns) => {
        const nextGraph = updateGraph({ nodes: ns, edges }, newG);
        return nextGraph.nodes;
      });
      setEdges((es) => {
        const nextGraph = updateGraph({ nodes, edges: es }, newG);
        return nextGraph.edges;
      });
    };
    return () => {
      if (ws.readyState == WebSocket.OPEN) ws.close();
    };
  }, [props]);

  const workflow_start =
    workflowsQuery.data?.find((workflow) => workflow.id == workflow_id)
      ?.start_time || "";
  const localGraph = loadGraph({ key: workflow_id });
  if (workflow_start && workflow_start != localGraph?.start_time) {
    deleteGraph({ key: workflow_id });
    localGraph == null;
  }

  const mergedGraph = (() => {
    const default_node_positions = bottomUpLayout(nodes, edges);
    if (localGraph === null)
      return {
        nodes: default_node_positions,
        edges: edges,
      };

    const mergedNodes = default_node_positions.map((node) => {
      return {
        ...node,
        position:
          localGraph.nodes.find(
            (oldNode) => oldNode.id === node.id && oldNode.data.pinned
          )?.position ?? node.position,
      };
    });
    const edgesMap = new Map();
    localGraph.edges.forEach((edge) => edgesMap.set(edge.id, edge));
    edges.forEach((edge) => edgesMap.set(edge.id, edge));
    const mergedEdges = [...edgesMap.values()];

    return {
      nodes: mergedNodes,
      edges: mergedEdges,
    };
  })();

  return (
    <Main
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      workflows={workflowsQuery.data ?? []}
      workflow_id={workflow_id}
      workflow_start=""
      infoProps={info}
      setInfo={setInfo}
    />
  );
}
