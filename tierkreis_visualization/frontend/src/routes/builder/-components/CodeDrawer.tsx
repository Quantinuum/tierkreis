import { python } from "@codemirror/lang-python";
import CodeMirror from "@uiw/react-codemirror";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Code2, Download, Play } from "lucide-react";

const downloadCode = (code: string) => {
  const blob = new Blob([code], { type: "text/x-python" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "graph.py";
  a.click();
  URL.revokeObjectURL(url);
};

/** Read-only, syntax-highlighted preview of the graph as Python-ish code.
 * Regenerates whenever the graph changes (one-way sync: graph -> code). */
export const CodeDrawer = (props: { code: string }) => (
  <Sheet>
    <SheetTrigger asChild>
      <Button variant="outline" size="sm">
        <Code2 /> Code
      </Button>
    </SheetTrigger>
    <SheetContent side="right" className="p-4 gap-2 sm:max-w-xl">
      <SheetHeader className="p-0">
        <SheetTitle>Generated code</SheetTitle>
      </SheetHeader>
      <div className="text-xs text-muted-foreground">
        Live preview only - stays in sync with the graph, but editing it here
        does not (yet) update the graph.
      </div>
      <div className="flex gap-2">
        <Button
          size="sm"
          variant="outline"
          onClick={() => downloadCode(props.code)}
        >
          <Download /> Download
        </Button>
        <Tooltip>
          <TooltipTrigger asChild>
            <span>
              <Button size="sm" variant="outline" disabled>
                <Play /> Run
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>Not implemented yet</TooltipContent>
        </Tooltip>
      </div>
      <div className="flex-1 overflow-auto rounded-md border">
        <CodeMirror
          value={props.code}
          extensions={[python()]}
          basicSetup={{ lineNumbers: true, foldGutter: false }}
          height="100%"
        />
      </div>
    </SheetContent>
  </Sheet>
);
