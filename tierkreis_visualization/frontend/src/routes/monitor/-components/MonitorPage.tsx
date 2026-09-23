import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useMonitoringSummaryQuery } from "@/data/api";
import {
  ArcElement,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Tooltip,
} from "chart.js";
import { useMemo } from "react";
import { Bar, Doughnut } from "react-chartjs-2";
import { Link } from "@tanstack/react-router";
import { RuntimeInfoCard } from "./RuntimeInfoCard";
import { RunHistoryHeatmap } from "./RunHistoryHeatmap";
import { WorkflowsTree } from "./WorkflowsTree";

ChartJS.register(
  ArcElement,
  BarElement,
  CategoryScale,
  LinearScale,
  Tooltip,
  Legend,
);

const STATUS_COLORS = {
  running: "#f59e0b",
  completed: "#22c55e",
  errored: "#ef4444",
  cancelled: "#94a3b8",
};

export default function MonitorPage() {
  const { data, isLoading, error } = useMonitoringSummaryQuery();

  const taskChartData = useMemo(
    () => ({
      labels: ["Running", "Completed", "Errored", "Cancelled"],
      datasets: [
        {
          data: [
            data?.tasks_running ?? 0,
            data?.tasks_completed ?? 0,
            data?.tasks_errored ?? 0,
            data?.tasks_cancelled ?? 0,
          ],
          backgroundColor: [
            STATUS_COLORS.running,
            STATUS_COLORS.completed,
            STATUS_COLORS.errored,
            STATUS_COLORS.cancelled,
          ],
        },
      ],
    }),
    [data],
  );

  const runsChartData = useMemo(
    () => ({
      labels: ["Total runs", "Runs with errors", "Active runs"],
      datasets: [
        {
          label: "Runs",
          data: [
            data?.total_runs ?? 0,
            data?.runs_with_errors ?? 0,
            data?.active_runs.length ?? 0,
          ],
          backgroundColor: ["#3b82f6", "#ef4444", "#f59e0b"],
        },
      ],
    }),
    [data],
  );

  return (
    <div className="p-8">
      <div className="flex items-center justify-between pb-8">
        <div className="text-4xl">Monitoring</div>
        <Link to="/builder" className="hover:underline">
          Graph builder (POC) &rarr;
        </Link>
      </div>

      {error && (
        <div className="text-nexus-red">Failed to load monitoring data.</div>
      )}
      {isLoading && <div className="text-muted-foreground">Loading...</div>}

      {data && (
        <div className="flex flex-col gap-6 max-w-4xl">
          <div className="grid grid-cols-4 gap-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  Active runs
                </CardTitle>
              </CardHeader>
              <CardContent className="text-2xl font-semibold">
                {data.active_runs.length}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  Total runs
                </CardTitle>
              </CardHeader>
              <CardContent className="text-2xl font-semibold">
                {data.total_runs}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  Runs with errors
                </CardTitle>
              </CardHeader>
              <CardContent className="text-2xl font-semibold text-nexus-red">
                {data.runs_with_errors}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  Avg task duration
                </CardTitle>
              </CardHeader>
              <CardContent className="text-2xl font-semibold">
                {data.avg_task_duration_seconds !== null
                  ? `${data.avg_task_duration_seconds.toFixed(1)}s`
                  : "N/A"}
              </CardContent>
            </Card>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <Card>
              <CardHeader>
                <CardTitle>Node status breakdown</CardTitle>
              </CardHeader>
              <CardContent>
                <Doughnut data={taskChartData} />
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Run summary</CardTitle>
              </CardHeader>
              <CardContent>
                <Bar
                  data={runsChartData}
                  options={{ scales: { y: { beginAtZero: true } } }}
                />
              </CardContent>
            </Card>
          </div>

          <RuntimeInfoCard />

          <RunHistoryHeatmap />

          <WorkflowsTree />
        </div>
      )}
    </div>
  );
}
