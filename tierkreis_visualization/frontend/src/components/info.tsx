import {
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { InfoProps } from "./types";

export function NodeInfo(props: { info: InfoProps }) {
  const restartHandler = async () => {
    const url = `http://localhost:8000/workflows/${props.info.workflow_id}/nodes/${props.info.node_location}/restart`;
    await fetch(url, { method: "POST" });
  };
  return (
    <DialogContent className="w-[90vw] h-[90vh]">
      <DialogHeader>
        <DialogTitle>
          {props.info.type} {props.info.workflow_id}:{props.info.node_location}
        </DialogTitle>
        <DialogDescription></DialogDescription>
      </DialogHeader>
      <div className="overflow-auto">
        <pre style={{ maxHeight: "65vh" }}>{props.info.content}</pre>
      </div>
      <button onClick={restartHandler}>Restart node</button>
    </DialogContent>
  );
}
