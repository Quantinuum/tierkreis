import { useEffect, useRef, useState } from "react";
import type { components } from "@/data/api_stubs";
import { fetchWorkerLogs } from "@/data/api";
import { Button } from "./ui/button";

type LogRecord = components["schemas"]["WorkerLogRecord"];

export function WorkerLogs({
  workflowId,
  nodeLocation,
}: {
  workflowId: string;
  nodeLocation?: string;
}) {
  const after = useRef(0);
  const [records, setRecords] = useState<LogRecord[]>([]);
  const [request, setRequest] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    let timer: number | undefined;
    const load = async () => {
      let hasMoreRecords = false;
      try {
        const page = await fetchWorkerLogs(
          workflowId,
          nodeLocation,
          after.current,
          controller.signal,
        );
        if (controller.signal.aborted) return;
        const last = page.records[page.records.length - 1];
        if (last) after.current = last.cursor;
        if (page.records.length > 0) {
          setRecords((existing) => [...existing, ...page.records]);
        }
        hasMoreRecords = page.next_cursor != null;
        setHasMore(hasMoreRecords);
        setFailed(false);
      } catch {
        if (controller.signal.aborted) return;
        setFailed(true);
      }
      setLoading(false);
      if (!hasMoreRecords) timer = window.setTimeout(load, 1000);
    };
    void load();
    return () => {
      controller.abort();
      window.clearTimeout(timer);
    };
  }, [workflowId, nodeLocation, request]);

  return (
    <div className="flex flex-col flex-1 min-h-0 gap-2">
      <div className="flex-1 overflow-auto rounded-lg border bg-gray-50 p-2">
        {failed && <p>Failed to fetch worker logs.</p>}
        {!failed && records.length === 0 && (
          <p>{loading ? "Loading logs…" : "No logs yet."}</p>
        )}
        {records.map((record) => (
          <div key={record.cursor} className="mb-2">
            <p className="text-xs text-gray-600">
              {record.collected_at} {nodeLocation === undefined && record.location}{" "}
              {record.executor} {record.source}
            </p>
            <pre className="whitespace-pre-wrap break-words">{record.message}</pre>
          </div>
        ))}
      </div>
      {hasMore && (
        <Button
          variant="outline"
          disabled={loading}
          onClick={() => {
            setLoading(true);
            setRequest((current) => current + 1);
          }}
        >
          Load more
        </Button>
      )}
    </div>
  );
}
