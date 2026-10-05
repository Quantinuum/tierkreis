import { InputHandleArray, OutputHandleArray } from "@/components/handles";
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

  let extra_classes = "";
  if (data.node_type === "input") {
    extra_classes = "rounded-b-lg";
  } else if (data.node_type === "output") {
    extra_classes = "rounded-t-lg";
  } else if (data.node_type === "const") {
    extra_classes = "rounded-lg ";
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
    <div
      className={
        "w-[80px] h-[80px] drag-handle p-4  shadow-sm flex items-center justify-center border-double border-sidebar-border border-3 " +
        bg_color(data.status) +
        " " +
        extra_classes
      }
      onClick={handleClick}
    >
      <DialogTrigger asChild>
        <div className="text-center font-semibold whitespace-nowrap overflow-hidden text-ellipsis">
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
          {name}
        </div>
      </DialogTrigger>
    </div>
  );
}
