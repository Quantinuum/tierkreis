import { useNavigate } from "@tanstack/react-router";

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
    <tr className="cursor-pointer hover:bg-gray-50" onClick={handleRowClick}>
      <td className="p-4 border-t-1">{r.name}</td>
      <td className="p-4 border-t-1">{r.id}</td>
      <td className="p-4 border-t-1">{d_display}</td>
    </tr>
  );
};
