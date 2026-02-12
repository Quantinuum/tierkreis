import { LoadingIndicator } from "@/components/StatusIndicator";
import { Button } from "@/components/ui/button";
import { WorkflowDisplay } from "@/data/api_types";
import { loc_parent } from "@/data/loc";
import { Link, useNavigate } from "@tanstack/react-router";
import { Download } from "lucide-react";

const NodeLink = (props: { wid: string; loc: string }) => {
  return (
    <Link
      to="/workflows/$wid/nodes/$loc"
      params={{ wid: props.wid, loc: loc_parent(props.loc) }}
      className="hover:underline ml-2"
    >
      {props.loc}
    </Link>
  );
};

const errorLinks = (wid: string, errors: string[]) => {
  return errors.map((x) => <NodeLink wid={wid} loc={x} />);
};
const getLogs = async (wid: string) => {
  window.open(`/api/workflows/${wid}/logs`, "_blank");
};

const logsLink = (wid: string) => {
  return (
    <Button
      className="cursor-pointer"
      size="icon"
      onClick={async () => await getLogs(wid)}
    >
      <Download />
    </Button>
  );
};

export const WorkflowsTableRow = (props: { row: WorkflowDisplay }) => {
  const navigate = useNavigate();
  const r = props.row;
  const handleRowClick = () => {
    navigate({
      to: "/workflows/$wid/nodes/$loc",
      params: { wid: r.id, loc: "-" },
    });
  };
  const d = new Date(r.start_time);
  const d_display = `${d.toDateString()}, ${d.toLocaleTimeString()}`;
  return (
    <tr className="hover:bg-gray-50">
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {r.name}
      </td>
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {r.id}
      </td>
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {d_display}
      </td>
      <td className="p-4 border-t-1">{errorLinks(r.id, r.errors)}</td>
      <td className="p-4 border-t-1 flex justify-center">{logsLink(r.id)}</td>
    </tr>
  );
};

export function LoadingRow() {
  return (
    <tr style={{ color: "rgba(0,0,0,0)" }}>
      <td className="p-4 border-t-1">
        <LoadingIndicator>
          <span></span>
        </LoadingIndicator>
      </td>

      <td className="p-4 border-t-1 ">00000000-0000-0000-0000-000000000000</td>
      <td className="p-4 border-t-1 ">Mon Dec 08 2025, 14:53:10</td>
      <td className="p-4 border-t-1"></td>
    </tr>
  );
}
