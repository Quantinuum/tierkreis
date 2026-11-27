import { LoadingIndicator } from "@/components/StatusIndicator";
import { Button } from "@/components/ui/button";
import { AttemptDisplay } from "@/data/api_types";
import { attemptStatus } from "@/data/workflows";
import { runUrl } from "@/data/api";
import { loc_parent } from "@/data/loc";
import { Link } from "@tanstack/react-router";
import { Download, AlertCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

function check_semver_matches(v: string, target: string) {
  return v === target;
}

const NodeLink = (props: { runId: string; loc: string; attempt: number }) => {
  return (
    <Link
      to="/runs/$wrid/nodes/$loc"
      params={{ wrid: props.runId, loc: loc_parent(props.loc) }}
      search={{ attempt: props.attempt }}
      className="hover:underline ml-2"
    >
      {props.loc}
    </Link>
  );
};

const errorLinks = (runId: string, attempt: number, errors: string[]) => {
  return errors.map((loc) => (
    <NodeLink key={loc} runId={runId} loc={loc} attempt={attempt} />
  ));
};

const getLogs = (runId: string, attempt: number) => {
  window.open(runUrl(runId, attempt, "logs"), "_blank");
};

const logsLink = (runId: string, attempt: number) => {
  return (
    <Button
      className="cursor-pointer"
      size="icon"
      aria-label={`Download logs for attempt ${attempt}`}
      onClick={() => getLogs(runId, attempt)}
    >
      <Download />
    </Button>
  );
};

export const AttemptsTableRow = (props: {
  row: AttemptDisplay;
  runId: string;
  targetVersion: string;
}) => {
  const { row, runId } = props;
  const status = attemptStatus(row);
  const date = row.start_time
    ? new Date(row.start_time).toLocaleString()
    : "Not started";
  return (
    <tr className="hover:bg-gray-50">
      <td className="p-4 border-t-1">
        <Link
          to="/runs/$wrid/nodes/$loc"
          params={{ wrid: runId, loc: "-" }}
          search={{ attempt: row.attempt }}
          className="hover:underline"
        >
          Attempt {row.attempt}
        </Link>
      </td>
      <td className={`p-4 border-t-1 ${status.className}`}>{status.label}</td>
      <td className="p-4 border-t-1">{date}</td>
      <td className="p-4 border-t-1">
        <div className="flex items-center gap-2">
          {row.tkr_version}
          {row.tkr_version &&
            !check_semver_matches(row.tkr_version, props.targetVersion) && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <AlertCircle className="w-4 h-4 text-nexus-red" />
                  </TooltipTrigger>
                  <TooltipContent>
                    This workflow was run with a different version of tierkreis
                    and may not be displayed correctly.
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            )}
        </div>
      </td>
      <td className="p-4 border-t-1">
        {errorLinks(runId, row.attempt, row.errors)}
      </td>
      <td className="p-4 border-t-1 text-center">
        {logsLink(runId, row.attempt)}
      </td>
    </tr>
  );
};

export function LoadingRow() {
  return (
    <tr>
      <td className="p-4">
        <LoadingIndicator>
          <span>Loading workflows...</span>
        </LoadingIndicator>
      </td>
    </tr>
  );
}
