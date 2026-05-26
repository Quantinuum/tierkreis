import { useInfoQuery, useWorkflowsQuery } from "@/data/api";
import { createFileRoute } from "@tanstack/react-router";
import { WorkflowsTable } from "./-components/table";

export const Route = createFileRoute("/workflows/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, error } = useWorkflowsQuery();
  const { data: infoData, error: infoError } = useInfoQuery();
  if (error) return <div>Error {error}</div>;
  console.log("Checking workflow version", infoData, infoError);
  return (
    <div className="p-8">
      <div className="text-4xl pb-8">Tierkreis workflows</div>
      <WorkflowsTable
        data={data ?? []}
        isLoading={data === undefined || infoData === undefined}
        targetVersion={infoData?.version ?? ""}
      />
    </div>
  );
}
