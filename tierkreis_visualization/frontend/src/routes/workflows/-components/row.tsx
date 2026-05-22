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

const TARGET_TKR_VERSION = "2.0.12";

function check_semver_matches(v: string | null | undefined, target: string) {
  if (!v) return false;
  const parts = v.split(".");
  if (parts.length < 3) return false;
  return v === target;
}

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
      <td className="p-4 border-t-1">
        <div className="flex items-center gap-2">
          {r.tkr_version}
          {r.tkr_version &&
            (() => {
              const isOutdated = !check_semver_matches(
                r.tkr_version,
                TARGET_TKR_VERSION,
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
      <td className="p-4 border-t-1 ">
        {new Date().toDateString()}, {new Date().toLocaleTimeString()}
      </td>
      <td className="p-4 border-t-1"></td>
      <td className="p-4 border-t-1"></td>
      <td className="p-4 border-t-1"></td>
    </tr>
  );
}
