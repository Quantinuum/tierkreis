import { loc_parent } from "@/data/loc";
import { Link, useNavigate } from "@tanstack/react-router";

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

export const WorkflowsTableRow = (props: { row: WorkflowRowData }) => {
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
    </tr>
  );
};
