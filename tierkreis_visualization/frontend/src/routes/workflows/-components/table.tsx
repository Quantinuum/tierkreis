import {
  createColumnHelper,
  SortingState,
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
} from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { WorkflowsTableRow } from "./row";
import { ColumnHeader } from "./header";
import { Workflow } from "@/data/api_types";

export function WorkflowsTable(props: { data: Workflow[] }) {
  const columnHelper = createColumnHelper<Workflow>();
  const columns = [
    columnHelper.accessor("name", { header: "Name" }),
    columnHelper.accessor("id", {
      header: "id",
      sortingFn: (x, y) => x.original.id_int - y.original.id_int,
    }),
    columnHelper.accessor("start_time", {
      header: "Start time",
      invertSorting: true,
    }),
    columnHelper.accessor("errors", {
      header: "Errors",
      sortingFn: (x, y) => y.original.errors.length - x.original.errors.length,
    }),
  ];

  const [sorting, setSorting] = useState<SortingState>([
    { id: "start_time", desc: false },
  ]);
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
        current_sort={sorting[0]}
        title={x.columnDef.header?.toString()}
        setSorting={setSorting}
      />
    ));

  const rows = table
    .getRowModel()
    .rows.map((row) => <WorkflowsTableRow row={row.original} />);

  return (
    <div className="p-8">
      <div className="text-4xl pb-8">Tierkreis workflows</div>
      <table
        cellSpacing="0"
        className="border-separate border-1 rounded-sm mb-4"
      >
        <thead>{heads}</thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}
