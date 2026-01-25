import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { DialogTrigger } from "@/components/ui/dialog";
import { fetchText } from "@/data/logs";
import { useQueryClient } from "@tanstack/react-query";
import { type NodeProps } from "@xyflow/react";
import { type BackendNode } from "./types";
import { OctagonAlert } from "lucide-react";

export function DefaultNode({ data }: NodeProps<BackendNode>) {
  const queryClient = useQueryClient();
  let name = data.title;
  if (name == "Function") {
    name = data.name;
  } else if (data.value) {
    name = data.value;
  }
  const bg_color = (status: string) => {
    switch (status) {
      case "Started":
        return "bg-chart-4";
      case "Finished":
        return "bg-emerald-600";
      case "Error":
        return "bg-red-400";
      default:
        return "bg-white";
    }
  };

  return (
    <Card className={"w-[180px] " + bg_color(data.status)}>
      "Hello"
      <DialogTrigger asChild>
        <div
          onClick={(event) => {
            //workaround to render errors
            const target = event.target as HTMLElement;
            if (target.closest("button") === null) {
              if (data.title == "Function") {
                const selectionKey = `${data.workflowId}:${data.node_location}:logs:${Date.now()}`;
                data.setInfo?.({
                  type: "Logs",
                  content: "Loading logs...",
                  selectionKey,
                });
                queryClient
                  .fetchQuery({
                    queryKey: ["logs", data.workflowId, data.node_location],
                    queryFn: () =>
                      fetchText(data.workflowId, data.node_location, "logs"),
                  })
                  .then((content) => {
                    data.setInfo?.((prev) =>
                      prev.selectionKey === selectionKey
                        ? { ...prev, content }
                        : prev
                    );
                  })
                  .catch((err: unknown) => {
                    data.setInfo?.((prev) =>
                      prev.selectionKey === selectionKey
                        ? {
                            ...prev,
                            content: `Failed to load logs: ${String(err)}`,
                          }
                        : prev
                    );
                  });
                return;
              }

              // Ensure we don't show stale function logs when a non-function node is clicked.
              const selectionKey = `${data.workflowId}:${data.node_location}:logs:${Date.now()}`;
              data.setInfo?.({
                type: "Logs",
                content: `No logs for "${data.title}" nodes.`,
                selectionKey,
              });
            }
          }}
        >
          <CardHeader>
            <CardTitle
              style={{ whiteSpace: "normal", wordBreak: "break-word" }}
            >
              {name}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <InputHandleArray
              handles={data.handles.inputs}
              id={data.id}
              isOpen={data.isTooltipOpen}
              hoveredId={data.hoveredId}
              setHoveredId={data.setHoveredId}
            />
            <div className="flex items-center justify-center">
              {data.status == "Error" && (
                <Button
                  size="sm"
                  variant="destructive"
                  style={{ zIndex: 5 }}
                  onClick={() => {
                    const selectionKey = `${data.workflowId}:${data.node_location}:errors:${Date.now()}`;
                    data.setInfo?.({
                      type: "Errors",
                      content: "Loading errors...",
                      selectionKey,
                    });
                    queryClient
                      .fetchQuery({
                        queryKey: ["errors", data.workflowId, data.node_location],
                        queryFn: () =>
                          fetchText(
                            data.workflowId,
                            data.node_location,
                            "errors"
                          ),
                      })
                      .then((content) => {
                        data.setInfo?.((prev) =>
                          prev.selectionKey === selectionKey
                            ? { ...prev, content }
                            : prev
                        );
                      })
                      .catch((err: unknown) => {
                        data.setInfo?.((prev) =>
                          prev.selectionKey === selectionKey
                            ? {
                                ...prev,
                                content: `Failed to load errors: ${String(err)}`,
                              }
                            : prev
                        );
                      });
                  }}
                >
                  <OctagonAlert />
                </Button>
              )}
            </div>
            <OutputHandleArray
              handles={data.handles.outputs}
              id={data.id}
              isOpen={data.isTooltipOpen}
              hoveredId={data.hoveredId}
              setHoveredId={data.setHoveredId}
            />
          </CardContent>
          <CardFooter
            className="flex justify-content justify-start"
            style={{ padding: "-5px" }}
          ></CardFooter>
        </div>
      </DialogTrigger>
    </Card>
  );
}
