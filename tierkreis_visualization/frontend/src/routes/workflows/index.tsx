import { useWorkflowsQuery } from "@/data/api";
import { createFileRoute } from "@tanstack/react-router";
import { WorkflowsTable } from "./-components/table";

export const Route = createFileRoute("/workflows/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, error } = useWorkflowsQuery();
  if (error) return <div>Error {error}</div>;

  return (
    <div className="p-8">
      <div className="text-4xl pb-8">Tierkreis workflows</div>
      <WorkflowsTable data={data ?? []} isLoading={data === undefined} />
    </div>
  );
}
