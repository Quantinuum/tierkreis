import NodePage from "@/routes/runs/_.$wrid.nodes.$loc/-components/NodePage";
import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

const pathSchema = z.object({ wrid: z.string(), loc: z.string() });
const validateSearch = z.object({
  attempt: z.number().int().nonnegative().default(0),
  openEvals: z.array(z.string()).default([]),
  openLoops: z.array(z.string()).default([]),
  openMaps: z.array(z.string()).default([]),
});

export const Route = createFileRoute("/runs/_/$wrid/nodes/$loc/")({
  component: RouteComponent,
  validateSearch,
  params: { parse: pathSchema.parse },
});

function RouteComponent() {
  const { wrid: workflow_run_id, loc } = Route.useParams();
  const { attempt, openEvals, openLoops, openMaps } = Route.useSearch();
  return (
    <NodePage
      key={JSON.stringify([workflow_run_id, attempt, loc])}
      workflow_id={workflow_run_id}
      attempt={attempt}
      node_location_str={loc}
      openEvals={openEvals}
      openLoops={openLoops}
      openMaps={openMaps}
    ></NodePage>
  );
}
