import {
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { InfoProps } from "./types";
import { fetchOutput, restartNode } from "@/data/api";
import { Button } from "./ui/button";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Download } from "lucide-react";
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
    const response = await fetchOutput(
      props.info.workflow_id,
      props.info.node_location,
      output_name,
    );
    const data = JSON.parse(response);
    const jsonString = JSON.stringify(data, null, 2);
    const blob = new Blob([jsonString], { type: "application/json" });

    const fileUrl = URL.createObjectURL(blob);
    window.open(fileUrl, "_blank");
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

  return (
    <DialogContent className="w-[90vw] h-[90vh]">
      <DialogHeader>
        <DialogTitle> {props.info.type}</DialogTitle>
        <DialogDescription>
          Started at: {props.info.started_time} Finished at:{" "}
          {props.info.finished_time ? props.info.finished_time : "N/A"}
        </DialogDescription>
        {props.info.output_names && props.info.output_names.length > 0 && (
          <div className="p-2 bg-gray-50 border border-gray-200 rounded-lg">
            <p className="text-sm text-gray-500 mb-2">Download Output Values</p>
            <div className="flex flex-row flex-wrap items-center gap-3">
              {props.info.output_names.map((name) => outputButton(name))}
            </div>
          </div>
        )}
      </DialogHeader>
      <div className="text-wrap overflow-auto h-9/10">{props.info.content}</div>

      {restartButton}
    </DialogContent>
  );
}
