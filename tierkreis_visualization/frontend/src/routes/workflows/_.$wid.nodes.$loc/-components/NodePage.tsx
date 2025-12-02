import {
  applyNodeChanges,
  Edge,
  NodeChange,
  useReactFlow,
} from "@xyflow/react";
import { InfoProps } from "@/components/types";
import { parseGraph } from "@/graph/parseGraph";
import "@xyflow/react/dist/style.css";
import { useCallback, useEffect, useState } from "react";
import { BackendNode } from "../../../../nodes/types";
import {
  evalQuery as createEvalQuery,
  listWorkflowsQuery,
  logsQuery,
} from "../../../../data/api";
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
  const { getIntersectingNodes } = useReactFlow();

  const workflowsQuery = listWorkflowsQuery();
  const logs = logsQuery(workflow_id);
  const evalQuery = createEvalQuery(workflow_id, [
    node_location_str,
    ...props.openEvals,
  ]);
  const evalData = evalQuery.data?.graphs ?? {};

  const [g, setG] = useLocalStorageState<Graph>(
    workflow_id + node_location_str,
    { defaultValue: { nodes: [], edges: [] } }
  );

  const onNodesChange = useCallback((changes: NodeChange<BackendNode>[]) => {
    setG((gSnapshot: Graph) => {
      const ns = applyNodeChanges(changes, gSnapshot.nodes);
      return { nodes: ns, edges: gSnapshot.edges };
    });
  }, []);

  const [info, setInfo] = useState<InfoProps>({
    type: "Logs",
    content: logs.data as string,
  });

  useEffect(() => {
    if (Object.keys(evalData).length == 0) return;
    let ns = [];
    let es = [];

    for (let loc in evalData) {
      ns.push(...evalData[loc].nodes);
      es.push(...evalData[loc].edges);
    }

    // Rewire inputs of open EVALs
    for (let e of es) {
      if (!Object.keys(evalData).includes(e.to_node)) continue;
      if (e.to_port === "body") continue;

      const newTarget = evalData[e.to_node].nodes.find(
        (x) => x.function_name === "input" && x.value === e.to_port
      );
      if (newTarget !== undefined) e.to_node = newTarget.id;
    }

    // TODO: rewire outputs of open EVALs
    for (let e of es) {
      if (!Object.keys(evalData).includes(e.from_node)) continue;

      const newSource = evalData[e.from_node].nodes.find(
        (x) => x.function_name === "output"
      );
      if (newSource !== undefined) e.from_node = newSource.id;
    }

    const newG = parseGraph({ nodes: ns, edges: es }, workflow_id);
    for (let n of newG.nodes) {
      if (Object.keys(evalData).includes(n.id)) {
        n.data.is_expanded = true;
      }
    }

    setG((oldG: Graph) => {
      return updateGraph(oldG, newG);
    });
  }, [props, workflow_id, node_location_str, evalData]);

  useEffect(() => {
    const url = `/api/workflows/${props.workflow_id}/nodes/${node_location_str}`;
    const ws = new WebSocket(url);
    ws.onmessage = () => evalQuery.refetch();
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
