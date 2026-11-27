import { LoadingIndicator } from "@/components/StatusIndicator";
import { Button } from "@/components/ui/button";
import { WorkflowDisplay } from "@/data/api_types";
import { loc_parent } from "@/data/loc";
import { Link, useNavigate } from "@tanstack/react-router";
import { Download, AlertCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

function check_semver_matches(v: string, target: string) {
  if (!v) return false;
  const parts = v.split(".");
  if (parts.length < 3) return false;
  return v === target;
}

const NodeLink = (props: { wrid: string; loc: string }) => {
  return (
    <Link
      to="/runs/$wrid/nodes/$loc"
      params={{ wrid: props.wrid, loc: loc_parent(props.loc) }}
      className="hover:underline ml-2"
    >
      {props.loc}
    </Link>
  );
};

const errorLinks = (wrid: string, errors: string[]) => {
  return errors.map((x) => <NodeLink wrid={wrid} loc={x} />);
};
const getLogs = async (wrid: string) => {
  window.open(`/api/runs/${wrid}/logs`, "_blank");
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

export const WorkflowsTableRow = (props: {
  row: WorkflowDisplay;
  targetVersion: string;
}) => {
  const navigate = useNavigate();
  const r = props.row;
  const first = r.runs[0]?.attempts[0]; // TODO
  const handleRowClick = () => {
    navigate({
      to: "/runs/$wrid/nodes/$loc",
      params: { wrid: r.runs[0]?.workflow_run_id ?? "-", loc: "-" }, // TODO
    });
  };
  if (!first || !first.start_time) return null; // TODO handle missing first attempt

  const d = new Date(first.start_time);
  const d_display = `${d.toDateString()}, ${d.toLocaleTimeString()}`;
  return (
    <tr className="hover:bg-gray-50">
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {r.name}
      </td>
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {r.workflow_id}
      </td>
      <td className="p-4 border-t-1 cursor-pointer" onClick={handleRowClick}>
        {d_display}
      </td>
      <td className="p-4 border-t-1">
        <div className="flex items-center gap-2">
          {first.tkr_version}
          {first.tkr_version &&
            (() => {
              const isOutdated = !check_semver_matches(
                first.tkr_version,
                props.targetVersion,
              );
              return isOutdated ? (
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <AlertCircle className="w-4 h-4 text-nexus-red" />
                    </TooltipTrigger>
                    <TooltipContent>
                      This workflow was run with a different version of
                      tierkreis and may not be displayed correctly.
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              ) : null;
            })()}
        </div>
      </td>
      <td className="p-4 border-t-1">
        {errorLinks(r.workflow_id, first.errors)}
      </td>
      <td className="p-4 border-t-1 flex justify-center">
        {logsLink(r.workflow_id)}
      </td>
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
      <td className="p-4 border-t-1 ">
        {new Date().toDateString()}, {new Date().toLocaleTimeString()}
      </td>
      <td className="p-4 border-t-1"></td>
      <td className="p-4 border-t-1"></td>
      <td className="p-4 border-t-1"></td>
    </tr>
  );
}
