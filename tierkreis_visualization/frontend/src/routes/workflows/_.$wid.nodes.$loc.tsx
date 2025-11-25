import App from "@/App";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/workflows/_/$wid/nodes/$loc")({
  component: RouteComponent,
});

function RouteComponent() {
  const { wid, loc } = Route.useParams();
  return <App workflow_id={wid} node_location_str={loc}></App>;
}
