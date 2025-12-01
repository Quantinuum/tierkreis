import NodePage from "@/routes/workflows/_.$wid.nodes.$loc/-components/NodePage";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/workflows/_/$wid/nodes/$loc/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { wid, loc } = Route.useParams();
  return (
    <NodePage
      key={wid + loc}
      workflow_id={wid}
      node_location_str={loc}
    ></NodePage>
  );
}
