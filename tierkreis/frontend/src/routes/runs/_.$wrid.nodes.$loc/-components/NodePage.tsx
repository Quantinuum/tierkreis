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
  runUrl,
} from "../../../../data/api";
import { updateGraph } from "@/graph/updateGraph";
import useLocalStorageState from "use-local-storage-state";
import { GraphView } from "./GraphView";
import { Graph } from "@/graph/models";
import { amalgamateGraphData } from "@/graph/amalgamateGraph";
import { PyGraph, WorkflowDisplay } from "@/data/api_types";
import { useParams } from "@tanstack/react-router";

type NodePageProps = {
  workflow_id: string;
  node_location_str: string;
  openEvals: string[];
  openLoops: string[];
  openMaps: string[];
};

export default function NodePage(props: NodePageProps & { attempt: number }) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;

  const workflowsQuery = useWorkflowsQuery();
  const logsQuery = useLogsQuery(workflow_id, props.attempt);
  const { data, error, refetch } = useEvalQuery(workflow_id, props.attempt, [
    node_location_str,
    ...props.openEvals,
    ...props.openLoops,
    ...props.openMaps,
  ]);

  useEffect(() => {
    const url = runUrl(
      workflow_id,
      props.attempt,
      `nodes/${encodeURIComponent(node_location_str)}`,
    );
    const ws = new WebSocket(url);
    ws.onmessage = () => {
      // current graph
      void refetch();
      // sidebar
      void workflowsQuery.refetch();
    };
    return () => ws.close();
  }, [
    workflow_id,
    props.attempt,
    node_location_str,
    refetch,
    workflowsQuery.refetch,
  ]);

  if (error)
    return (
      <div role="alert">Failed to load workflow attempt: {String(error)}</div>
    );
  if (workflowsQuery.error)
    return (
      <div role="alert">
        Failed to load workflows: {String(workflowsQuery.error)}
      </div>
    );

  return (
    <NodeGraph
      {...props}
      storageKey={JSON.stringify([
        workflow_id,
        props.attempt,
        node_location_str,
      ])}
      graphs={data?.graphs}
      workflows={workflowsQuery.data ?? []}
      initialContent={
        logsQuery.error ? "Failed to fetch logs." : logsQuery.data || "No logs."
      }
    />
  );
}

export function NodeGraph(
  props: NodePageProps & {
    storageKey: string;
    graphs?: Record<string, PyGraph>;
    workflows: WorkflowDisplay[];
    initialContent: string;
  },
) {
  const workflow_id = props.workflow_id;
  const node_location_str = props.node_location_str;
  const { wid: workflowDefinitionId } = useParams({ strict: false });
  const [stored_graph, setG] = useLocalStorageState<Graph>(props.storageKey, {
    defaultValue: { nodes: [], edges: [] },
  });

  const onNodesChange = useCallback(
    (changes: NodeChange<BackendNode>[]) => {
      setG((gSnapshot: Graph) => {
        const ns = applyNodeChanges(changes, gSnapshot.nodes);
        return { nodes: ns, edges: gSnapshot.edges };
      });
    },
    [setG],
  );

  const [info, setInfo] = useState<InfoProps>({
    type: "Logs",
    content: props.initialContent,
    workflow_id,
    node_location: node_location_str,
    has_error: false,
  });

  useEffect(() => {
    const evalData = props.graphs ?? {};
    if (Object.keys(evalData).length == 0) return;
    const { nodes, edges } = amalgamateGraphData(
      evalData,
      workflowDefinitionId
        ? [...props.openEvals, ...props.openLoops, ...props.openMaps]
        : props.openEvals,
      workflowDefinitionId ? [] : props.openLoops,
      workflowDefinitionId ? [] : props.openMaps,
    );

    const newG = parseGraph(
      { nodes, edges },
      workflow_id,
      props.openEvals,
      props.openLoops,
      props.openMaps,
    );
    for (const node of newG.nodes) {
      if (node.parentId === node_location_str) node.parentId = undefined;
    }
    setG((oldG: Graph) => updateGraph(oldG, newG));
  }, [
    props.graphs,
    props.openEvals,
    props.openLoops,
    props.openMaps,
    workflowDefinitionId,
    workflow_id,
    node_location_str,
    setG,
  ]);

  return (
    <GraphView
      key={props.storageKey}
      nodes={(stored_graph.nodes ?? []).map((node) =>
        node.parentId === node_location_str
          ? { ...node, parentId: undefined }
          : node,
      )}
      edges={stored_graph.edges ?? []}
      onNodesChange={onNodesChange}
      workflows={props.workflows}
      workflow_id={workflow_id}
      loc={node_location_str}
      infoProps={info}
      setInfo={setInfo}
    />
  );
}
