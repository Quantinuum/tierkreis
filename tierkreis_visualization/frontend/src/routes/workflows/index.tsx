import { $api } from "@/lib/api";
import { createFileRoute } from "@tanstack/react-router";
import { WorkflowsTable } from "./-components/table";

export const Route = createFileRoute("/workflows/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, error } = $api.useQuery("get", "/api/workflows/");
  if (error) return <div>Error {error}</div>;

  return <WorkflowsTable data={data ?? []} />;
}
