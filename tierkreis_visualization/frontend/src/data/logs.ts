import { fetchClient } from "@/lib/api";

export const fetchLogs = async (workflow_id: string) => {
  const res = await fetchClient.GET("/api/workflows/{workflow_id}/logs", {
    params: { path: { workflow_id } },
    parseAs: "text",
  });
  return res.data ?? "No logs.";
};

export const fetchErrors = async (
  workflow_id: string,
  node_location_str: string
) => {
  const params = { path: { workflow_id, node_location_str } };
  const res = await fetchClient.GET(
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/errors",
    { params, parseAs: "text" }
  );
  return res.data ?? "No errors.";
};
