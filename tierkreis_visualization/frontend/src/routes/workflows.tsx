import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/workflows")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/workflows"!</div>;
}
