import {
  applyEdgeChanges,
  applyNodeChanges,
  Edge,
  NodeChange,
  EdgeChange,
} from "@xyflow/react";

import { InfoProps } from "@/components/types";
import { parseGraph } from "@/graph/parseGraph";
import "@xyflow/react/dist/style.css";
import { useCallback, useEffect, useState } from "react";
import { BackendNode } from "../../../../nodes/types";
import { listWorkflowsQuery, logsQuery } from "../../../../data/api";
import { updateGraph } from "@/graph/updateGraph";
import useLocalStorageState from "use-local-storage-state";
import { GraphView } from "./GraphView";

export default function NodePage(props: {
  workflow_id: string;
  node_location_str: string;
  openEvals: string[];
}) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;
  const [nodes, setNodes] = useLocalStorageState<BackendNode[]>(
    workflow_id + node_location_str,
    { defaultValue: [] }
  );
  const [edges, setEdges] = useState<Edge[]>([]);

  const onNodesChange = useCallback(
    (changes: NodeChange<BackendNode>[]) =>
      setNodes((nodesSnapshot) => applyNodeChanges(changes, nodesSnapshot)),
    []
  );
  const onEdgesChange = useCallback(
    (changes: EdgeChange<Edge>[]) =>
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
        const nextGraph = updateGraph({ nodes: ns ?? [], edges }, newG);
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
  }, [props, workflow_id, node_location_str]);

  return (
    <GraphView
      key={workflow_id + node_location_str}
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      workflows={workflowsQuery.data ?? []}
      workflow_id={workflow_id}
      infoProps={info}
      setInfo={setInfo}
    />
  );
}
