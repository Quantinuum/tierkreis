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
import { useErrors, useLogs } from "@/data/logs";
import { type NodeProps } from "@xyflow/react";
import { type BackendNode } from "./types";
import { OctagonAlert } from "lucide-react";

export function DefaultNode({ data }: NodeProps<BackendNode>) {
  const { data: logs } = useLogs(
    data.workflowId,
    data.node_location,
    data.title
  );
  const { data: errors } = useErrors(
    data.workflowId,
    data.node_location,
    data.status
  );
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
      <DialogTrigger asChild>
        <div
          onClick={(event) => {
            //workaround to render errors
            const target = event.target as HTMLElement;
            if (target.closest("button") === null) {
              if (data.title == "Function") {
                data.setInfo?.({ type: "Logs", content: logs ? logs : "" });
              }
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
                  onClick={() =>
                    data.setInfo?.({
                      type: "Errors",
                      content: errors ? errors : "",
                    })
                  }
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
