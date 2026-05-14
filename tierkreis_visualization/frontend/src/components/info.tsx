import {
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { InfoProps } from "./types";
import { restartNode } from "@/data/api";
import { Button } from "./ui/button";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Download } from "lucide-react";
import { useEffect, useState } from "react";
export function NodeInfo(props: { info: InfoProps; closer: () => void }) {
  const navigate = useNavigate();
  const search = useSearch({ strict: false });

  const restartHandle = async () => {
    const invalid = await restartNode(
      props.info.workflow_id,
      props.info.node_location,
    );

    const isValid = (openNode: string) => {
      for (const invalidatedNode of invalid) {
        if (openNode.startsWith(invalidatedNode)) return false;
      }
      return true;
    };

    const openEvals = search.openEvals?.filter(isValid);
    const openLoops = search.openLoops?.filter(isValid);
    const openMaps = search.openMaps?.filter(isValid);

    navigate({ to: ".", search: { openEvals, openLoops, openMaps } });
    props.closer();
  };
  const getOutput = async (output_name: string) => {
    window.open(
      `/api/workflows/${props.info.workflow_id}/nodes/${props.info.node_location}/outputs/${output_name}`,
      "_blank",
    );
  };

  const getMetadata = async () => {
    window.open(
      `/api/workflows/${props.info.workflow_id}/nodes/${props.info.node_location}/metadata`,
      "_blank",
    );
  };

  const restartButton =
    props.info.type === "Logs" ? (
      <Button className="cursor-pointer mt-2" onClick={restartHandle}>
        Restart
      </Button>
    ) : (
      <></>
    );
  const outputButton = (outputName: string) => {
    return (
      <Button
        className="cursor-pointer"
        onClick={async () => await getOutput(outputName)}
      >
        {outputName} <Download className="inline-block ml-2 mb-1" />
      </Button>
    );
  };
  const metadataButton = () => {
    return (
      <Button
        className="cursor-pointer pl-0"
        variant="link"
        onClick={async () => await getMetadata()}
      >
        Download Executor Metadata
      </Button>
    );
  };

  const [duration, setDuration] = useState<number | null>(null);
  useEffect(() => {
    const start = props.info.started_time
      ? new Date(props.info.started_time)
      : undefined;
    const end = props.info.finished_time
      ? new Date(props.info.finished_time)
      : undefined;
    const updateDuration = () => {
      const endTime = end || new Date();
      if (start) {
        setDuration(Math.floor((endTime.getTime() - start.getTime()) / 1000));
      }
    };
    const interval = setInterval(updateDuration, 1000);
    return () => clearInterval(interval);
  }, [props.info.started_time, props.info.finished_time]);

  return (
    <DialogContent className="w-[90vw] h-[90vh] flex flex-col">
      <DialogHeader>
        <DialogTitle>
          {" "}
          {props.info.type}{" "}
          {props.info.task_name && ` - ${props.info.task_name}`}
        </DialogTitle>

        <DialogDescription>
          Started at:{" "}
          {props.info.started_time ? props.info.started_time : "N/A"} Finished
          at: {props.info.finished_time ? props.info.finished_time : "N/A"}{" "}
          {!props.info.has_error && (
            <>Duration: {duration !== null ? `${duration}s` : "N/A"}</>
          )}
          <br />
          {props.info.task_name && metadataButton()}
        </DialogDescription>
        {props.info.output_names &&
          props.info.output_names.length > 0 &&
          props.info.finished_time && (
            <div className="p-2 bg-gray-50 border border-gray-200 rounded-lg">
              <p className="text-xs text-gray-500 mb-2">Outputs</p>
              <div className="flex flex-row flex-wrap items-center gap-3">
                {props.info.output_names.map((name) => outputButton(name))}
              </div>
            </div>
          )}
      </DialogHeader>
      {props.info.content !== "Failed to fetch output." &&
        !props.info.type?.includes("if/else") && (
          <div className="text-wrap overflow-y-auto overflow-x-hidden flex-1 mt-2 p-2 bg-gray-50 border border-gray-200 rounded-lg">
            <p className="text-sm text-gray-500 mb-2">{props.info.type}</p>
            <pre className="whitespace-pre-wrap break-words">
              {props.info.content}
            </pre>
          </div>
        )}

      {restartButton}
    </DialogContent>
  );
}
