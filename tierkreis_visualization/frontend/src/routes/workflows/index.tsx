import { $api } from "@/lib/api";
import { createFileRoute, Link } from "@tanstack/react-router";
import {
  createColumnHelper,
  useReactTable,
  getCoreRowModel,
  Cell,
  flexRender,
} from "@tanstack/react-table";
import { useMemo } from "react";
export const Route = createFileRoute("/workflows/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, error, isLoading } = $api.useQuery("get", "/workflows/all");
  const defaultData = useMemo(() => [], []);

  type WorkflowData = NonNullable<typeof data>[number];
  const columnHelper = createColumnHelper<WorkflowData>();
  const columns = [
    columnHelper.accessor("id", {
      header: "id",
      cell: (info) => (
        <Link to={"/workflows/$wid"} params={{ wid: info.getValue() }}>
          {info.getValue()}
        </Link>
      ),
      footer: (props) => props.column.id,
    }),
    columnHelper.accessor("name", {
      header: () => "Name",
      cell: (info) => info.getValue(),
    }),
    columnHelper.accessor("start_time", {
      header: () => "start_time",
      cell: (info) => info.getValue(),
    }),
  ];

  const table = useReactTable({
    columns,
    data: data ?? defaultData,
    getCoreRowModel: getCoreRowModel(),
    debugTable: true,
  });

  const cell = (c: Cell<WorkflowData, unknown>) => (
    <td> {flexRender(c.column.columnDef.cell, c.getContext())}</td>
  );

  const rows = table.getRowModel().rows.map((row) => {
    return <tr>{row.getVisibleCells().map(cell)}</tr>;
  });

  if (isLoading) return <div>...</div>;
  if (error || data === undefined) return <div>Error {error}</div>;
  return (
    <table>
      <tbody>{rows}</tbody>
    </table>
  );
}
