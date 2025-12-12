import {
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { InfoProps } from "./types";

export function NodeInfo(props: { info: InfoProps }) {
  return (
    <DialogContent className="w-[90vw] h-[90vh]">
      <DialogHeader>
        <DialogTitle>{props.info.type}</DialogTitle>
        <DialogDescription></DialogDescription>
      </DialogHeader>
      <div className="text-wrap">{props.info.content}</div>
    </DialogContent>
  );
}
