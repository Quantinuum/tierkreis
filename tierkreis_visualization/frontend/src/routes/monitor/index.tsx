import { createFileRoute } from "@tanstack/react-router";
import MonitorPage from "./-components/MonitorPage";

export const Route = createFileRoute("/monitor/")({
  component: MonitorPage,
});
