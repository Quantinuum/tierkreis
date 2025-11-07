import {
  DialogContent,
  DialogClose,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { InfoProps } from "./types";
import { URL } from "@/data/constants";
import { Button } from "./ui/button";

export function NodeInfo(props: { info: InfoProps }) {
  const restartHandler = async () => {
    const url = `${URL}/${props.info.workflowId}/nodes/${props.info.node_location}/restart`;
    await fetch(url, { method: "POST" });
  };
  return (
    <DialogContent className="w-[90vw] h-[90vh]">
      <DialogHeader>
        <DialogTitle>
          {props.info.type} {props.info.workflowId}:{props.info.node_location}
        </DialogTitle>
        <DialogDescription></DialogDescription>
      </DialogHeader>
      <div className="overflow-auto">
        <pre style={{ maxHeight: "65vh" }}>{props.info.content}</pre>
      </div>
      <div>
        <DialogClose asChild>
          <Button onClick={restartHandler} size="sm">
            Restart node
          </Button>
        </DialogClose>
      </div>
    </DialogContent>
  );
}
