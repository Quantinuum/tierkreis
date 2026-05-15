import { InputHandleArray, OutputHandleArray } from "@/components/handles";
import { DialogTrigger } from "@/components/ui/dialog";
import { type NodeProps } from "@xyflow/react";
import { type BackendNode } from "./types";
import { bg_color } from "@/components/colors";

export function DefaultNode({ data }: NodeProps<BackendNode>) {
  let name = data.title;
  if (name == "Function") {
    name = data.name;
  } else if (data.value) {
    name = data.value;
  }

  const handleClick = async () => {
    const workflow_id = data.workflowId;
    const node_location = data.node_location;
    if (data.node_type === "function") {
      return;
    } else if (data.node_type === "const") {
      return;
    } else if (data.node_type === "input") {
      return;
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
      return;
    } else {
      data.node_type satisfies never;
    }
  };

  return (
    <div
      className={
        "w-[80px] h-[80px] drag-handle p-4  shadow-sm flex items-center justify-center rounded-[18px] " +
        bg_color(data.status)
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
