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

export function NodeInfo(props: { info: InfoProps; closer: () => void }) {
  const navigate = useNavigate();
  const search = useSearch({ strict: false });

  const restartHandle = async () => {
    const invalid = await restartNode(
      props.info.workflow_id,
      props.info.node_location
    );

    const isValid = (openNode: string) => {
      for (const invalidatedNode of invalid) {
        if (openNode.startsWith(invalidatedNode)) return false
      }
      return true;
    }

    const openEvals = search.openEvals?.filter(isValid);
    const openLoops = search.openLoops?.filter(isValid);
    const openMaps = search.openMaps?.filter(isValid);

    navigate({ to: ".", search: { openEvals, openLoops, openMaps } });
    props.closer();
  };

  return (
    <DialogContent className="w-[90vw] h-[90vh]">
      <DialogHeader>
        <DialogTitle> {props.info.type}</DialogTitle>
        <DialogDescription></DialogDescription>
      </DialogHeader>
      <div className="text-wrap overflow-auto h-9/10">{props.info.content}</div>
      <Button className="cursor-pointer mt-2" onClick={restartHandle}>
        Restart
      </Button>
    </DialogContent>
  );
}
