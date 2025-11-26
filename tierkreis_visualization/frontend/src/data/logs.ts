import { useQuery } from "@tanstack/react-query";

const fetchText = async (
  workflowId: string,
  node_location: string,
  type: "errors" | "logs"
) => {
  const url = `/api/workflows/${workflowId}/nodes/${node_location}/${type}`;
  const response = await fetch(url, {
    method: "GET",
    headers: { Accept: "application/text" },
  });
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return response.text();
};

export const useLogs = (
  workflowId: string,
  node_location: string,
  node_title: string
) => {
  return useQuery({
    queryKey: ["logs", workflowId, node_location],
    queryFn: () => fetchText(workflowId, node_location, "logs"),
    enabled: !!workflowId && !!node_location && node_title == "Function",
  });
};

export const useErrors = (
  workflowId: string,
  node_location: string,
  status: "Not started" | "Started" | "Error" | "Finished"
) => {
  return useQuery({
    queryKey: ["errors", workflowId, node_location],
    queryFn: () => fetchText(workflowId, node_location, "errors"),
    enabled: !!workflowId && !!node_location && status === "Error",
  });
};
