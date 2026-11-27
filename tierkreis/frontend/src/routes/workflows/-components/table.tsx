import {
  createColumnHelper,
  SortingState,
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
} from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { LoadingRow, WorkflowsTableRow } from "./row";
import { ColumnHeader } from "./header";
import { WorkflowDisplay } from "@/data/api_types";

export function WorkflowsTable(props: {
  data: WorkflowDisplay[];
  isLoading: boolean;
  targetVersion: string;
}) {
  const columnHelper = createColumnHelper<WorkflowDisplay>();
  const columns = [
    columnHelper.accessor("name", { header: "Name" }),
    columnHelper.accessor("runs.workflow_run_id", {
      header: "id",
      sortingFn: (x, y) =>
        (x.original.runs[0]?.workflow_run_id ?? "").localeCompare(
          y.original.runs[0]?.workflow_run_id ?? "",
        ),
    }),
    columnHelper.accessor("runs.attempts.start_time", {
      header: "Start time",
      invertSorting: true,
    }),
    columnHelper.accessor("runs.attempts.tkr_version", {
      header: "TKR version",
    }),
    columnHelper.accessor("runs.attempts.errors", {
      header: "Errors",
      sortingFn: (x, y) =>
        (y.original.runs[0]?.attempts[0]?.errors.length ?? 0) -
        (x.original.runs[0]?.attempts[0]?.errors.length ?? 0),
    }),
    {
      header: "Logs",
    },
  ];

  const defaultSort = { id: "start_time", desc: false };
  const [sorting, setSorting] = useState<SortingState>([defaultSort]);
  const defaultData = useMemo(() => [], []);
  const table = useReactTable({
    columns,
    data: props.data ?? defaultData,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    debugTable: true,
    state: { sorting },
    onSortingChange: setSorting,
  });

  const heads = table
    .getAllColumns()
    .map((x) => (
      <ColumnHeader
        id={x.id}
        current_sort={sorting[0] ?? defaultSort}
        title={x.columnDef.header?.toString()}
        setSorting={setSorting}
      />
    ));

  const rows = table
    .getRowModel()
    .rows.map((row) => (
      <WorkflowsTableRow
        row={row.original}
        targetVersion={props.targetVersion}
      />
    ));

  const body = props.isLoading ? [<LoadingRow />] : rows;

  return (
    <table cellSpacing="0" className="border-separate border-1 rounded-sm mb-4">
      <thead>{heads}</thead>
      <tbody>{body}</tbody>
    </table>
  );
}
