import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { startNewRun, useWorkflowInputNamesQuery } from "@/data/api";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

// Below this many input ports, render one text field per port; otherwise
// fall back to a raw JSON textarea (still prefilled with the port names).
const MAX_DYNAMIC_FIELDS = 5;

const parseFieldValue = (raw: string): unknown => {
  try {
    return JSON.parse(raw);
  } catch {
    return raw;
  }
};

export const NewRunDialog = (props: {
  workflowId: string;
  workflowName: string;
}) => {
  const [open, setOpen] = useState(false);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [inputsText, setInputsText] = useState("{}");
  const [error, setError] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { data: inputNames } = useWorkflowInputNamesQuery(props.workflowId);

  const useDynamicForm = (inputNames?.length ?? 0) <= MAX_DYNAMIC_FIELDS;

  useEffect(() => {
    if (!open || !inputNames) return;
    setFields(Object.fromEntries(inputNames.map((name) => [name, ""])));
    setInputsText(
      JSON.stringify(
        Object.fromEntries(inputNames.map((name) => [name, null])),
        null,
        2,
      ),
    );
  }, [open, inputNames]);

  const submit = async () => {
    let inputs: Record<string, unknown>;
    if (useDynamicForm) {
      inputs = Object.fromEntries(
        Object.entries(fields).map(([name, value]) => [
          name,
          parseFieldValue(value),
        ]),
      );
    } else {
      try {
        inputs = JSON.parse(inputsText);
      } catch {
        setError('Inputs must be valid JSON, e.g. {"value": 5}');
        return;
      }
    }
    try {
      await startNewRun(props.workflowId, inputs);
      setError(null);
      setOpen(false);
      await queryClient.invalidateQueries({
        queryKey: ["monitor", "workflows"],
      });
    } catch {
      setError("Failed to start new run.");
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline">
          New run
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Start new run of {props.workflowName}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-2">
          {useDynamicForm ? (
            (inputNames?.length ?? 0) === 0 ? (
              <div className="text-sm text-muted-foreground">
                This Workflow takes no inputs.
              </div>
            ) : (
              inputNames?.map((name) => (
                <div key={name} className="flex flex-col gap-1">
                  <label
                    className="text-sm text-muted-foreground"
                    htmlFor={`input-${name}`}
                  >
                    {name}
                  </label>
                  <input
                    id={`input-${name}`}
                    className="border rounded-md p-2 font-mono text-sm"
                    value={fields[name] ?? ""}
                    onChange={(e) =>
                      setFields((prev) => ({ ...prev, [name]: e.target.value }))
                    }
                  />
                </div>
              ))
            )
          ) : (
            <>
              <label className="text-sm text-muted-foreground" htmlFor="inputs">
                Inputs (JSON object, keyed by input port name)
              </label>
              <textarea
                id="inputs"
                className="border rounded-md p-2 font-mono text-sm h-48"
                value={inputsText}
                onChange={(e) => setInputsText(e.target.value)}
              />
            </>
          )}
          {error && <div className="text-nexus-red text-sm">{error}</div>}
          <Button onClick={submit}>Start</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
};
