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
import { fetchOutput, fetchOutputs } from "@/data/api";
import { loc_parent } from "@/data/loc";
import { bg_color } from "@/components/colors";

export function SimpleNode({ data }: NodeProps<BackendNode>) {
  let name = data.title;
  if (data.value) {
    name = data.value;
  }

  const handleClick = async () => {
    const workflow_id = data.workflowId;
    const node_location = data.node_location;
    if (data.node_type === "const") {
      const content = await fetchOutput(
        data.workflowId,
        data.node_location,
        "value",
      );
      data.setInfo?.({
        type: "Constant value",
        content,
        workflow_id,
        node_location,
        started_time: data.started_time,
        finished_time: data.finished_time,
        output_names: data.output_names,
        has_error: data.has_error,
      });
    } else if (data.node_type === "input") {
      const content = await fetchOutput(
        data.workflowId,
        data.node_location,
        name,
      );
      data.setInfo?.({
        type: "Input",
        content,
        workflow_id,
        node_location,
        started_time: data.started_time,
        finished_time: data.finished_time,
        output_names: data.output_names,
        has_error: data.has_error,
      });
    } else if (data.node_type === "output") {
      const parent = loc_parent(data.node_location);
      const content = await fetchOutputs(data.workflowId, parent);
      data.setInfo?.({
        type: "Output",
        content,
        workflow_id,
        node_location,
        started_time: data.started_time,
        finished_time: data.finished_time,
        output_names: data.output_names,
        has_error: data.has_error,
      });
    } else {
      return;
    }
  };

  return (
    <Card
      className={"w-[180px] " + bg_color(data.status)}
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
