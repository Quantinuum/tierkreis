import { bg_color } from "@/components/colors";
import { BackendNode } from "@/nodes/types";
import { useMemo } from "react";

type TimedNode = {
  id: string;
  name: string;
  status: string;
  start: number;
  end: number;
};

// A lightweight Gantt-style view of node execution timings, built from
// data already loaded for the graph view (no extra API calls).
export const Timeline = (props: { nodes: BackendNode[] }) => {
  const timed: TimedNode[] = useMemo(() => {
    const now = Date.now();
    return props.nodes
      .filter((n) => n.data.started_time)
      .map((n) => ({
        id: n.id,
        name: n.data.name || n.data.title,
        status: n.data.status,
        start: new Date(n.data.started_time).getTime(),
        end: n.data.finished_time
          ? new Date(n.data.finished_time).getTime()
          : now,
      }))
      .sort((a, b) => a.start - b.start);
  }, [props.nodes]);

  if (timed.length === 0) {
    return (
      <div className="p-8 text-muted-foreground">
        No node timing data available yet.
      </div>
    );
  }

  const rangeStart = Math.min(...timed.map((t) => t.start));
  const rangeEnd = Math.max(...timed.map((t) => t.end));
  const rangeMs = Math.max(rangeEnd - rangeStart, 1);

  return (
    <div className="p-6 overflow-auto h-full">
      <div className="text-sm text-muted-foreground mb-4">
        {new Date(rangeStart).toLocaleString()} &rarr;{" "}
        {new Date(rangeEnd).toLocaleString()}
      </div>
      <div className="flex flex-col gap-2">
        {timed.map((t) => {
          const leftPct = ((t.start - rangeStart) / rangeMs) * 100;
          const widthPct = Math.max(((t.end - t.start) / rangeMs) * 100, 0.5);
          const durationS = Math.max(Math.round((t.end - t.start) / 1000), 0);
          return (
            <div key={t.id} className="flex items-center gap-3">
              <div className="w-48 shrink-0 truncate text-sm" title={t.name}>
                {t.name}
              </div>
              <div className="relative flex-1 h-6 bg-muted rounded-sm">
                <div
                  className={`absolute h-6 rounded-sm ${bg_color(t.status)}`}
                  style={{ left: `${leftPct}%`, width: `${widthPct}%` }}
                  title={`${durationS}s`}
                />
              </div>
              <div className="w-16 shrink-0 text-right text-xs text-muted-foreground">
                {durationS}s
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
