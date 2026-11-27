import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { useWorkflowGraphQuery, useWorkflowsQuery } from "@/data/api";
import { NodeGraph } from "@/routes/runs/_.$wrid.nodes.$loc/-components/NodePage";

const validateSearch = z.object({
  openEvals: z.array(z.string()).default([]),
  openLoops: z.array(z.string()).default([]),
  openMaps: z.array(z.string()).default([]),
});

export const Route = createFileRoute("/workflows/_/$wid/nodes/$loc/")({
  component: RouteComponent,
  validateSearch,
  params: { parse: z.object({ wid: z.string(), loc: z.string() }).parse },
});

function RouteComponent() {
  const { wid, loc } = Route.useParams();
  const { openEvals, openLoops, openMaps } = Route.useSearch();
  const workflows = useWorkflowsQuery();
  const graph = useWorkflowGraphQuery(wid, [
    loc,
    ...openEvals,
    ...openLoops,
    ...openMaps,
  ]);

  if (workflows.error || graph.error) {
    return (
      <div role="alert">
        Failed to load workflow graph: {String(graph.error ?? workflows.error)}
      </div>
    );
  }

  return (
    <NodeGraph
      key={JSON.stringify([wid, loc])}
      workflow_id={wid}
      node_location_str={loc}
      openEvals={openEvals}
      openLoops={openLoops}
      openMaps={openMaps}
      storageKey={JSON.stringify(["workflow", wid, loc])}
      graphs={graph.data?.graphs}
      workflows={workflows.data ?? []}
      initialContent="This workflow has not been run."
    />
  );
}
