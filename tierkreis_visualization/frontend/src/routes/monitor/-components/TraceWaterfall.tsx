import { useRunTraceQuery } from "@/data/api";
import { TraceSpan } from "@/data/api_types";
import { useMemo } from "react";

const STATUS_BG: Record<TraceSpan["status"], string> = {
  "Not started": "bg-card",
  Started: "bg-amber-500",
  Finished: "bg-nexus-green",
  Error: "bg-nexus-red",
};

export const TraceWaterfall = (props: { runId: string; attempt: number }) => {
  const { data, isLoading, error } = useRunTraceQuery(
    props.runId,
    props.attempt,
  );

  const timed = useMemo(() => {
    const spans = data ?? [];
    const start = (s: TraceSpan) =>
      s.running_time ?? s.queued_time ?? s.scheduled_time ?? s.complete_time;
    const end = (s: TraceSpan) =>
      s.complete_time ??
      s.error_time ??
      s.cancelled_time ??
      new Date().toISOString();

    return spans
      .filter((s) => start(s))
      .map((s) => ({
        span: s,
        start: new Date(start(s) as string).getTime(),
        end: new Date(end(s)).getTime(),
      }))
      .sort((a, b) => a.start - b.start);
  }, [data]);

  if (isLoading)
    return <div className="text-muted-foreground p-8">Loading...</div>;
  if (error)
    return <div className="text-nexus-red p-8">Failed to load trace.</div>;
  if (timed.length === 0) {
    return (
      <div className="text-muted-foreground p-8">
        No trace data available yet.
      </div>
    );
  }

  const rangeStart = Math.min(...timed.map((t) => t.start));
  const rangeEnd = Math.max(...timed.map((t) => t.end));
  const rangeMs = Math.max(rangeEnd - rangeStart, 1);

  return (
    <div className="flex flex-col gap-2 p-4">
      <div className="text-sm text-muted-foreground">
        {new Date(rangeStart).toLocaleString()} &rarr;{" "}
        {new Date(rangeEnd).toLocaleString()}
      </div>
      {timed.map(({ span, start, end }) => {
        const leftPct = ((start - rangeStart) / rangeMs) * 100;
        const widthPct = Math.max(((end - start) / rangeMs) * 100, 0.5);
        const durationS = Math.max(Math.round((end - start) / 1000), 0);
        return (
          <div key={span.location} className="flex items-center gap-3">
            <div
              className="w-32 shrink-0 truncate text-sm font-mono"
              title={span.location}
            >
              {span.location || "root"}
            </div>
            <div className="relative flex-1 h-6 bg-muted rounded-sm">
              <div
                className={`absolute h-6 rounded-sm ${STATUS_BG[span.status]}`}
                style={{ left: `${leftPct}%`, width: `${widthPct}%` }}
                title={span.error ?? `${durationS}s`}
              />
            </div>
            <div className="w-16 shrink-0 text-right text-xs text-muted-foreground">
              {durationS}s
            </div>
          </div>
        );
      })}
    </div>
  );
};
