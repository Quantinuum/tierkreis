import {
  createColumnHelper,
  SortingState,
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
} from "@tanstack/react-table";
import { useState } from "react";
import { LoadingRow, AttemptsTableRow } from "./row";
import { ColumnHeader } from "./header";
import { AttemptDisplay, WorkflowDisplay } from "@/data/api_types";
import { attemptStatus, latestAttempt } from "@/data/workflows";

function AttemptsTable(props: {
  data: AttemptDisplay[];
  runId: string;
  targetVersion: string;
}) {
  const columnHelper = createColumnHelper<AttemptDisplay>();
  const columns = [
    columnHelper.accessor("attempt", { header: "Attempt" }),
    columnHelper.accessor((attempt) => attemptStatus(attempt).label, {
      id: "status",
      header: "Status",
    }),
    columnHelper.accessor("start_time", {
      header: "Start time",
      invertSorting: true,
    }),
    columnHelper.accessor("tkr_version", { header: "TKR version" }),
    columnHelper.accessor("errors", {
      header: "Errors",
      sortingFn: (x, y) => y.original.errors.length - x.original.errors.length,
    }),
    columnHelper.display({ id: "logs", header: "Logs" }),
  ];
  const defaultSort = { id: "start_time", desc: false };
  const [sorting, setSorting] = useState<SortingState>([defaultSort]);
  const table = useReactTable({
    columns,
    data: props.data,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    state: { sorting },
    onSortingChange: setSorting,
  });

  return (
    <table className="w-full border-separate text-sm">
      <thead>
        <tr>
          {table.getAllColumns().map((column) => (
            <ColumnHeader
              key={column.id}
              id={column.id}
              current_sort={sorting[0] ?? defaultSort}
              title={column.columnDef.header?.toString()}
              setSorting={setSorting}
            />
          ))}
        </tr>
      </thead>
      <tbody>
        {table.getRowModel().rows.map((row) => (
          <AttemptsTableRow
            key={row.original.attempt}
            row={row.original}
            runId={props.runId}
            targetVersion={props.targetVersion}
          />
        ))}
      </tbody>
    </table>
  );
}

export function WorkflowsTable(props: {
  data: WorkflowDisplay[];
  isLoading: boolean;
  targetVersion: string;
}) {
  if (props.isLoading)
    return (
      <table aria-label="Loading workflows">
        <tbody>
          <LoadingRow />
        </tbody>
      </table>
    );
  if (!props.data.length)
    return <p className="text-muted-foreground">No workflows yet.</p>;
  return (
    <div className="flex flex-col gap-2">
      {props.data.map((workflow) => {
        const latest = latestAttempt(workflow);
        const status = latest && attemptStatus(latest);
        return (
          <details key={workflow.workflow_id} className="border rounded-md">
            <summary className="cursor-pointer p-4 hover:bg-muted/40">
              <span className="font-semibold" title={workflow.workflow_id}>
                {workflow.name ?? "Unnamed Workflow"}
              </span>
              <span className="ml-3 text-sm text-muted-foreground">
                {workflow.runs.length} Run
                {workflow.runs.length === 1 ? "" : "s"}
              </span>
              {status && (
                <span className={`ml-3 text-sm ${status.className}`}>
                  Last: {status.label}
                </span>
              )}
            </summary>
            <div className="border-t p-3">
              {workflow.runs.length === 0 && (
                <p className="text-sm text-muted-foreground">No runs yet.</p>
              )}
              {workflow.runs.map((run) => (
                <section key={run.workflow_run_id} className="mb-3 last:mb-0">
                  <h2 className="bg-muted/40 p-2 text-sm font-medium break-all">
                    Run {run.workflow_run_id}
                  </h2>
                  <div className="overflow-x-auto">
                    <AttemptsTable
                      data={run.attempts}
                      runId={run.workflow_run_id}
                      targetVersion={props.targetVersion}
                    />
                  </div>
                </section>
              ))}
            </div>
          </details>
        );
      })}
    </div>
  );
}
