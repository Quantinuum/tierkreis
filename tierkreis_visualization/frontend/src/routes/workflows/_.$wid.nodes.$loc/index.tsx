import GraphView from "@/routes/workflows/_.$wid.nodes.$loc/-components/GraphView";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/workflows/_/$wid/nodes/$loc/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { wid, loc } = Route.useParams();
  return <GraphView workflow_id={wid} node_location_str={loc}></GraphView>;
}
