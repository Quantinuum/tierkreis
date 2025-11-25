import { $api } from "@/lib/api";
import { createFileRoute, Link } from "@tanstack/react-router";
import {
  createColumnHelper,
  useReactTable,
  getCoreRowModel,
  Cell,
  flexRender,
  SortingState,
  getSortedRowModel,
} from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { ArrowDownNarrowWide, ArrowUpNarrowWide, ArrowUp } from "lucide-react";

export const Route = createFileRoute("/workflows/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, error, isLoading } = $api.useQuery("get", "/api/workflows/");
  const defaultData = useMemo(() => [], []);

  type WorkflowData = NonNullable<typeof data>[number];
  const columnHelper = createColumnHelper<WorkflowData>();
  const columns = [
    columnHelper.accessor("id", {
      header: "id",
      cell: (info) => (
        <Link
          className="cursor-pointer hover:underline"
          to={"/workflows/$wid/nodes/$loc"}
          params={{ wid: info.getValue(), loc: "-" }}
        >
          {info.getValue()}
        </Link>
      ),
      footer: (props) => props.column.id,
      sortingFn: (x, y) => x.original.id_int - y.original.id_int,
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

  const [sorting, setSorting] = useState<SortingState>([
    { id: "start_time", desc: true },
  ]);
  const table = useReactTable({
    columns,
    data: data ?? defaultData,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    debugTable: true,
    state: { sorting },
    onSortingChange: setSorting,
  });
  const toggleSorting = (col: string) => {
    const old = sorting[0];
    if (old.id == col) return setSorting([{ id: col, desc: !old.desc }]);
    return setSorting([{ id: col, desc: false }]);
  };

  const cell = (c: Cell<WorkflowData, unknown>) => (
    <td className="p-1">
      {flexRender(c.column.columnDef.cell, c.getContext())}
    </td>
  );

  const rows = table.getRowModel().rows.map((row) => {
    return (
      <tr className="odd:bg-gray-100 even:bg-white">
        {row.getVisibleCells().map(cell)}
      </tr>
    );
  });

  const sortingButton = (col: string) => {
    const old = sorting[0];
    if (old.id === col && old.desc) {
      return (
        <ArrowDownNarrowWide onClick={(_) => toggleSorting(col)}>
          sort
        </ArrowDownNarrowWide>
      );
    } else if (old.id === col && !old.desc) {
      return (
        <ArrowUpNarrowWide onClick={(_) => toggleSorting(col)}>
          sort
        </ArrowUpNarrowWide>
      );
    } else {
      return <ArrowUp onClick={(_) => toggleSorting(col)}> sort</ArrowUp>;
    }
  };

  if (isLoading) return <div>...</div>;
  if (error || data === undefined) return <div>Error {error}</div>;
  return (
    <div className="flex flex-col justify-center items-center">
      <div className="text-4xl py-4">Tierkreis workflows</div>
      <table>
        <thead>
          <th className="text-left py-2">
            <div className="flex select-none">
              {sortingButton("id")}
              <div>id</div>
            </div>
          </th>
          <th className="text-left py-2">
            <div className="flex select-none">
              {sortingButton("name")}
              <div>Name</div>
            </div>
          </th>
          <th className="text-left py-2">
            <div className="flex select-none">
              {sortingButton("start_time")}
              <div>Start time</div>
            </div>
          </th>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}
