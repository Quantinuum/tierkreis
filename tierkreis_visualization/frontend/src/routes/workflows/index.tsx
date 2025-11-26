import { $api } from "@/lib/api";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
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
  const navigate = useNavigate();
  const { data, error, isLoading } = $api.useQuery("get", "/api/workflows/");
  const defaultData = useMemo(() => [], []);

  type WorkflowData = NonNullable<typeof data>[number];
  const columnHelper = createColumnHelper<WorkflowData>();
  const columns = [
    columnHelper.accessor("id", {
      header: "id",
      sortingFn: (x, y) => x.original.id_int - y.original.id_int,
    }),
    columnHelper.accessor("name", {
      header: () => "Name",
    }),
    columnHelper.accessor("start_time", {
      header: () => "start_time",
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

  const handleRowClick = (wid: string) => {
    navigate({ to: "/workflows/$wid/nodes/$loc", params: { wid, loc: "-" } });
  };

  const rows = table.getRowModel().rows.map((row) => {
    const r = row.original;
    const h = () => handleRowClick(r.id);
    const d = new Date(r.start_time);
    const d_display = `${d.toDateString()}, ${d.toLocaleTimeString()}`;
    return (
      <tr className="cursor-pointer hover:bg-gray-50" onClick={h}>
        <td className="p-4 border-t-1">{r.name}</td>
        <td className="p-4 border-t-1">{r.id}</td>
        <td className="p-4 border-t-1">{d_display}</td>
      </tr>
    );
  });

  const sortingButton = (col: string) => {
    const old = sorting[0];
    if (old.id === col && old.desc) {
      return (
        <ArrowDownNarrowWide
          className="cursor-pointer mr-2"
          onClick={(_) => toggleSorting(col)}
        >
          sort
        </ArrowDownNarrowWide>
      );
    } else if (old.id === col && !old.desc) {
      return (
        <ArrowUpNarrowWide
          className="cursor-pointer mr-2"
          onClick={(_) => toggleSorting(col)}
        >
          sort
        </ArrowUpNarrowWide>
      );
    } else {
      return (
        <ArrowUp
          className="cursor-pointer mr-2"
          onClick={(_) => toggleSorting(col)}
        >
          sort
        </ArrowUp>
      );
    }
  };

  if (isLoading) return <div>...</div>;
  if (error || data === undefined) return <div>Error {error}</div>;
  return (
    <div className="p-8">
      <div className="text-4xl pb-8">Tierkreis workflows</div>
      <table
        cellSpacing="0"
        className="border-separate border-1 rounded-sm mb-4"
      >
        <thead>
          <th className="text-left p-4">
            <div className="flex select-none">
              {sortingButton("name")}
              <div>Name</div>
            </div>
          </th>
          <th className="text-left p-4">
            <div className="flex select-none">
              {sortingButton("id")}
              <div>id</div>
            </div>
          </th>
          <th className="text-left p-4">
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
