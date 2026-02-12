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
import { type NodeProps } from "@xyflow/react";
import { type BackendNode } from "./types";
import { OctagonAlert } from "lucide-react";
import {
  fetchErrors,
  fetchNodeLogs,
  fetchOutput,
  fetchOutputs,
} from "@/data/api";
import { loc_parent } from "@/data/loc";

export function DefaultNode({ data }: NodeProps<BackendNode>) {
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
        return "bg-red-400 bg-repeat";
      default:
        return "bg-white";
    }
  };

  const handleClick = async () => {
    const workflow_id = data.workflowId;
    const node_location = data.node_location;
    if (data.node_type === "function") {
      const content = await fetchNodeLogs(data.workflowId, data.node_location);
      data.setInfo?.({
        type: "Logs",
        content,
        workflow_id,
        node_location,
        started_time: data.started_time,
        finished_time: data.finished_time,
        output_names: data.output_names,
        has_error: data.has_error,
      });
    } else if (data.node_type === "const") {
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
    } else if (data.node_type === "eifelse") {
      data.setInfo?.({
        type: "Eager if/else",
        content: "",
        workflow_id,
        node_location,
        has_error: data.has_error,
      });
    } else if (data.node_type === "ifelse") {
      data.setInfo?.({
        type: "Lazy if/else",
        content: "",
        workflow_id,
        node_location,
        started_time: data.started_time,
        finished_time: data.finished_time,
        output_names: data.output_names,
        has_error: data.has_error,
      });
    } else if (data.node_type === "eval") {
      return;
    } else if (data.node_type === "map") {
      return;
    } else if (data.node_type === "loop") {
      return;
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
      data.node_type satisfies never;
    }
  };
  const handleErrorClick = async () => {
    const errors = await fetchErrors(data.workflowId, data.node_location);
    data.setInfo?.({
      type: "Errors",
      content: errors,
      workflow_id: data.workflowId,
      node_location: data.node_location,
      started_time: data.started_time,
      finished_time: data.finished_time,
      has_error: data.has_error,
    });
  };

  return (
    <Card
      className={"w-[180px] " + bg_color(data.status)}
      onClick={handleClick}
      // style={{
      //   backgroundImage:
      //     "radial-gradient(circle, #7f1d1d 1px, transparent 1px)",
      //   backgroundSize: "20px 20px",
      // }}
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
            <div className="flex items-center justify-center">
              {data.status == "Error" && (
                <Button
                  size="sm"
                  variant="destructive"
                  style={{ zIndex: 5 }}
                  onClick={handleErrorClick}
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
