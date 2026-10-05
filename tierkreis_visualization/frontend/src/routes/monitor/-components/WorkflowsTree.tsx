import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { startNewAttempt, useWorkflowsSummaryQuery } from "@/data/api";
import { AttemptSummary, WorkflowSummary } from "@/data/api_types";
import { NewRunDialog } from "./NewRunDialog";
import { buildHistoryDays, toWeeks } from "./history";
import { HistoryDot } from "./HistoryDot";

const WORKFLOW_HISTORY_WEEKS = 8;

const attemptStatus = (attempt: AttemptSummary) => {
  if (attempt.error_time)
    return { label: "Error", className: "text-nexus-red" };
  if (attempt.cancelled_time)
    return { label: "Cancelled", className: "text-muted-foreground" };
  if (attempt.complete_time)
    return { label: "Success", className: "text-nexus-green" };
  return { label: "Running", className: "text-amber-500" };
};

const latestAttempt = (workflow: WorkflowSummary) => {
  const attempts = workflow.runs.flatMap((r) => r.attempts);
  return attempts.reduce<AttemptSummary | null>((latest, attempt) => {
    if (!latest || attempt.started_time > latest.started_time) return attempt;
    return latest;
  }, null);
};

const AttemptRow = (props: { runId: string; attempt: AttemptSummary }) => {
  const status = attemptStatus(props.attempt);
  return (
    <tr className="border-b">
      <td className="py-1 pl-8">
        <Link
          className="hover:underline"
          to="/workflows/$wid/nodes/$loc"
          params={{ wid: props.runId, loc: "-" }}
          search={{
            openEvals: [],
            openLoops: [],
            openMaps: [],
            view: "timeline",
          }}
        >
          Attempt {props.attempt.attempt}
        </Link>
      </td>
      <td className={`py-1 ${status.className}`}>{status.label}</td>
      <td className="py-1">
        {new Date(props.attempt.started_time).toLocaleString()}
      </td>
    </tr>
  );
};

const RunSection = (props: { runId: string; attempts: AttemptSummary[] }) => {
  const queryClient = useQueryClient();
  const restart = async () => {
    await startNewAttempt(props.runId);
    await queryClient.invalidateQueries({ queryKey: ["monitor", "workflows"] });
  };

  return (
    <>
      <tr className="border-b bg-muted/40">
        <td className="py-1 pl-4 font-medium" colSpan={2}>
          Run {props.runId}
        </td>
        <td className="py-1">
          <Button size="sm" variant="ghost" onClick={restart}>
            New attempt
          </Button>
        </td>
      </tr>
      {props.attempts.map((attempt) => (
        <AttemptRow
          key={attempt.attempt}
          runId={props.runId}
          attempt={attempt}
        />
      ))}
    </>
  );
};

/** Compact history strip: a Workflow's own "history section", GitHub-style. */
const WorkflowHistoryStrip = (props: { workflow: WorkflowSummary }) => {
  const entries = useMemo(
    () =>
      props.workflow.runs.flatMap((run) =>
        run.attempts
          .filter((attempt) => attempt.started_time)
          .map((attempt) => ({
            time: attempt.started_time,
            errored: !!attempt.error_time,
          })),
      ),
    [props.workflow],
  );
  const days = useMemo(
    () => buildHistoryDays(entries, WORKFLOW_HISTORY_WEEKS),
    [entries],
  );
  const weeks = useMemo(() => toWeeks(days), [days]);

  return (
    <div className="flex gap-0.5">
      {weeks.map((week, i) => (
        <div key={week[0]?.key ?? i} className="flex flex-col gap-0.5">
          {week.map((day) => (
            <HistoryDot key={day.key} day={day} sizeClass="w-2 h-2" />
          ))}
        </div>
      ))}
    </div>
  );
};

const WorkflowRow = (props: { workflow: WorkflowSummary }) => {
  const [expanded, setExpanded] = useState(false);
  const { workflow } = props;
  const runCount = workflow.runs.length;
  const latest = latestAttempt(workflow);
  const latestStatus = latest ? attemptStatus(latest) : null;

  return (
    <div className="border rounded-md">
      <button
        type="button"
        onClick={() => setExpanded((e) => !e)}
        className="w-full flex items-center justify-between gap-4 p-3 text-left hover:bg-muted/40"
      >
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-muted-foreground">{expanded ? "▼" : "▶"}</span>
          <span className="font-semibold truncate">
            {workflow.name ?? workflow.workflow_id}
          </span>
          <span className="text-sm text-muted-foreground">
            {runCount} run{runCount === 1 ? "" : "s"}
          </span>
          {latestStatus && (
            <span className={`text-sm ${latestStatus.className}`}>
              Last: {latestStatus.label}
            </span>
          )}
        </div>
        <div className="flex items-center gap-4 shrink-0">
          <WorkflowHistoryStrip workflow={workflow} />
          <div onClick={(e) => e.stopPropagation()}>
            <NewRunDialog
              workflowId={workflow.workflow_id}
              workflowName={workflow.name ?? workflow.workflow_id}
            />
          </div>
        </div>
      </button>
      {expanded && (
        <div className="border-t p-3">
          {runCount === 0 ? (
            <div className="text-muted-foreground text-sm">No runs yet.</div>
          ) : (
            <table className="w-full text-sm">
              <tbody>
                {workflow.runs.map((run) => (
                  <RunSection
                    key={run.run_id}
                    runId={run.run_id}
                    attempts={run.attempts}
                  />
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
};

export const WorkflowsTree = () => {
  const { data, isLoading, error } = useWorkflowsSummaryQuery();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Workflows, runs &amp; attempts</CardTitle>
      </CardHeader>
      <CardContent>
        {error && (
          <div className="text-nexus-red">Failed to load workflows.</div>
        )}
        {isLoading && <div className="text-muted-foreground">Loading...</div>}
        {data?.length === 0 && (
          <div className="text-muted-foreground">No workflows yet.</div>
        )}
        <div className="flex flex-col gap-2">
          {data?.map((workflow) => (
            <WorkflowRow key={workflow.workflow_id} workflow={workflow} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
};
