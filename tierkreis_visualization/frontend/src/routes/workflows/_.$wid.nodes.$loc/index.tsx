import NodePage from "@/routes/workflows/_.$wid.nodes.$loc/-components/NodePage";
import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

const pathSchema = z.object({ wid: z.string(), loc: z.string() });
const validateSearch = z.object({
  openEvals: z.array(z.string()).default([]),
  openLoops: z.array(z.string()).default([]),
  openMaps: z.array(z.string()).default([]),
});

export const Route = createFileRoute("/workflows/_/$wid/nodes/$loc/")({
  component: RouteComponent,
  validateSearch,
  params: { parse: pathSchema.parse },
});

function RouteComponent() {
  const { wid, loc } = Route.useParams();
  const { openEvals, openLoops, openMaps } = Route.useSearch();
  return (
    <NodePage
      key={wid + loc}
      workflow_id={wid}
      node_location_str={loc}
      openEvals={openEvals ?? []}
      openLoops={openLoops ?? []}
      openMaps={openMaps ?? []}
    ></NodePage>
  );
}
