import { applyNodeChanges, Edge, NodeChange } from "@xyflow/react";
import { InfoProps } from "@/components/types";
import { parseGraph } from "@/graph/parseGraph";
import "@xyflow/react/dist/style.css";
import { useCallback, useEffect, useState } from "react";
import { BackendNode } from "../../../../nodes/types";
import { evalQuery, listWorkflowsQuery, logsQuery } from "../../../../data/api";
import { updateGraph } from "@/graph/updateGraph";
import useLocalStorageState from "use-local-storage-state";
import { GraphView } from "./GraphView";
import { Graph } from "./models";

export default function NodePage(props: {
  workflow_id: string;
  node_location_str: string;
  openEvals: string[];
}) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;

  const workflowsQuery = listWorkflowsQuery();
  const logs = logsQuery(workflow_id);
  const nodeQuery = evalQuery(workflow_id, [
    node_location_str,
    ...props.openEvals,
  ]);
  const nodeData = nodeQuery.data?.graphs ?? {};

  const [g, setG] = useLocalStorageState<Graph>(
    workflow_id + node_location_str,
    { defaultValue: { nodes: [], edges: [] } }
  );

  const onNodesChange = useCallback(
    (changes: NodeChange<BackendNode>[]) =>
      setG((gSnapshot: Graph) => {
        const ns = applyNodeChanges(changes, gSnapshot.nodes);
        return { nodes: ns, edges: gSnapshot.edges };
      }),
    []
  );

  const [info, setInfo] = useState<InfoProps>({
    type: "Logs",
    content: logs.data as string,
  });

  useEffect(() => {
    if (Object.keys(nodeData).length == 0) return;
    const newG = parseGraph(nodeData[node_location_str], props.workflow_id);
    setG((oldG: Graph) => updateGraph(oldG, newG));
  }, [props, workflow_id, node_location_str, nodeData]);

  useEffect(() => {
    const url = `/api/workflows/${props.workflow_id}/nodes/${node_location_str}`;
    const ws = new WebSocket(url);
    ws.onmessage = (event) => nodeQuery.refetch();
    return () => {
      if (ws.readyState == WebSocket.OPEN) ws.close();
    };
  }, [props, workflow_id, node_location_str]);

  return (
    <GraphView
      key={workflow_id + node_location_str}
      nodes={g.nodes ?? []}
      edges={g.edges ?? []}
      onNodesChange={onNodesChange}
      workflows={workflowsQuery.data ?? []}
      workflow_id={workflow_id}
      infoProps={info}
      setInfo={setInfo}
    />
  );
}
