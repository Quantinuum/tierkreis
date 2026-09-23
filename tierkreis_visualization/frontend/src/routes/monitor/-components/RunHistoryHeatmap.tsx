import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useWorkflowsSummaryQuery } from "@/data/api";
import { useMemo } from "react";
import {
  GREEN_LEVEL_COLORS,
  RED_LEVEL_COLORS,
  buildHistoryDays,
  toWeeks,
} from "./history";
import { HistoryDot } from "./HistoryDot";

const WEEKS = 20;

/** GitHub/Airflow-style calendar: a circle marks each day at least one Workflow ran, red if the latest run that day failed. */
export const RunHistoryHeatmap = () => {
  const { data, isLoading } = useWorkflowsSummaryQuery();

  const entries = useMemo(
    () =>
      (data ?? []).flatMap((workflow) =>
        workflow.runs.flatMap((run) =>
          run.attempts
            .filter((attempt) => attempt.started_time)
            .map((attempt) => ({
              time: attempt.started_time,
              errored: !!attempt.error_time,
            })),
        ),
      ),
    [data],
  );

  const days = useMemo(() => buildHistoryDays(entries, WEEKS), [entries]);
  const weeks = useMemo(() => toWeeks(days), [days]);
  const hasAnyRuns = days.some((d) => d.count > 0);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Run history</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading && <div className="text-muted-foreground">Loading...</div>}
        {!isLoading && (
          <div className="flex flex-col gap-2">
            {!hasAnyRuns && (
              <div className="text-muted-foreground text-sm">
                No runs yet - start a run below to see it appear here.
              </div>
            )}
            <div className="flex gap-1 overflow-x-auto pb-1">
              {weeks.map((week, i) => (
                <div key={week[0]?.key ?? i} className="flex flex-col gap-1">
                  {week.map((day) => (
                    <HistoryDot key={day.key} day={day} sizeClass="w-3 h-3" />
                  ))}
                </div>
              ))}
            </div>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <span>No runs</span>
              <div
                className={`w-3 h-3 rounded-full ${GREEN_LEVEL_COLORS[0]}`}
              />
              <span className="ml-2">Ran</span>
              <div
                className={`w-3 h-3 rounded-full ${GREEN_LEVEL_COLORS[4]}`}
              />
              <span className="ml-2">Failed</span>
              <div className={`w-3 h-3 rounded-full ${RED_LEVEL_COLORS[4]}`} />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
