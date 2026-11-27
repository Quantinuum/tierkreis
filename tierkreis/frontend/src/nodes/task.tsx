import { bg_color } from "@/components/colors";
import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { DialogTrigger } from "@/components/ui/dialog";
import { type NodeProps } from "@xyflow/react";
import { type BackendNode } from "./types";

export function TaskNode({ data }: NodeProps<BackendNode>) {
  let name = data.title;
  if (name == "Function") {
    name = data.name;
  }

  const handleClick = () => {
    if (data.node_type !== "function") return;
    data.setInfo?.({
      type: data.status === "Error" ? "Errors" : "Logs",
      content: "",
      workflow_id: data.workflowId,
      node_location: data.node_location,
      started_time: data.started_time,
      finished_time: data.finished_time,
      output_names: data.output_names,
      node_inputs: data.node_inputs,
      has_error: data.has_error,
      task_name: data.name,
    });
  };

  return (
    <Card
      className={"w-[180px] drag-handle " + bg_color(data.status)}
      onClick={handleClick}
    >
      <DialogTrigger asChild>
        <div>
          <CardHeader>
            <CardTitle className="whitespace-nowrap overflow-hidden text-ellipsis">
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
