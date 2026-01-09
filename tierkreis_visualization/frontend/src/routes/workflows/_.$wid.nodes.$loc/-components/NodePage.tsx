import { applyNodeChanges, NodeChange } from "@xyflow/react";
import { InfoProps } from "@/components/types";
import { parseGraph } from "@/graph/parseGraph";
import "@xyflow/react/dist/style.css";
import { useCallback, useEffect, useState } from "react";
import { BackendNode } from "../../../../nodes/types";
import {
  useEvalQuery,
  useWorkflowsQuery,
  useLogsQuery,
} from "../../../../data/api";
import { updateGraph } from "@/graph/updateGraph";
import useLocalStorageState from "use-local-storage-state";
import { GraphView } from "./GraphView";
import { Graph } from "@/graph/models";
import { amalgamateGraphData } from "@/graph/amalgamateGraph";

export default function NodePage(props: {
  workflow_id: string;
  node_location_str: string;
  openEvals: string[];
  openLoops: string[];
  openMaps: string[];
}) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;

  const workflowsQuery = useWorkflowsQuery();
  const logsQuery = useLogsQuery(workflow_id);
  const { data, refetch } = useEvalQuery(workflow_id, [
    node_location_str,
    ...props.openEvals,
    ...props.openLoops,
    ...props.openMaps,
  ]);

  const [g, setG] = useLocalStorageState<Graph>(
    workflow_id + node_location_str,
    { defaultValue: { nodes: [], edges: [] } }
  );

  const onNodesChange = useCallback(
    (changes: NodeChange<BackendNode>[]) => {
      setG((gSnapshot: Graph) => {
        const ns = applyNodeChanges(changes, gSnapshot.nodes);
        return { nodes: ns, edges: gSnapshot.edges };
      });
    },
    [setG]
  );

  const [info, setInfo] = useState<InfoProps>({
    type: "Logs",
    content: logsQuery.error
      ? "Failed to fetch logs."
      : logsQuery.data || "No logs.",
    workflow_id,
    node_location: node_location_str,
  });

  useEffect(() => {
    const evalData = data?.graphs ?? {};
    if (Object.keys(evalData).length == 0) return;
    const { nodes, edges } = amalgamateGraphData(
      evalData,
      props.openEvals,
      props.openLoops,
      props.openMaps
    );

    const newG = parseGraph(
      { nodes, edges },
      workflow_id,
      props.openEvals,
      props.openLoops,
      props.openMaps
    );
    setG((oldG: Graph) => updateGraph(oldG, newG));
  }, [props, workflow_id, node_location_str, setG, data]);

  useEffect(() => {
    const url = `/api/workflows/${workflow_id}/nodes/${node_location_str}`;
    const ws = new WebSocket(url);
    ws.onmessage = () => refetch();
    return () => ws.close();
  }, [workflow_id, node_location_str, refetch]);

  return (
    <GraphView
      key={workflow_id + node_location_str}
      nodes={g.nodes ?? []}
      edges={g.edges ?? []}
      onNodesChange={onNodesChange}
      workflows={workflowsQuery.data ?? []}
      workflow_id={workflow_id}
      loc={node_location_str}
      infoProps={info}
      setInfo={setInfo}
    />
  );
}
