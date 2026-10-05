import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { useWorkflowPortsQuery, useWorkflowsQuery } from "@/data/api";
import { WorkflowDisplay } from "@/data/api_types";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { Blocks } from "lucide-react";
import { FAKE_WORKERS, TKR_TYPES, type BuilderNodeData } from "./model";

const Section = (props: { title: string; children: React.ReactNode }) => (
  <div className="flex flex-col gap-2">
    <div className="text-sm font-semibold">{props.title}</div>
    {props.children}
  </div>
);

const TypeSelect = (props: {
  value: string;
  onChange: (value: string) => void;
}) => (
  <select
    value={props.value}
    onChange={(e) => props.onChange(e.target.value)}
    className="h-8 rounded-md border bg-transparent px-2 text-sm"
  >
    {TKR_TYPES.map((t) => (
      <option key={t} value={t}>
        {t}
      </option>
    ))}
  </select>
);

/** Name + TKR[...] type, used for input/output ports. */
const PortInlineAdd = (props: {
  placeholder: string;
  buttonLabel: string;
  onAdd: (name: string, tkrType: string) => void;
}) => {
  const [value, setValue] = useState("");
  const [tkrType, setTkrType] = useState<string>(TKR_TYPES[0]);
  return (
    <div className="flex gap-2">
      <Input
        placeholder={props.placeholder}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        className="h-8"
      />
      <TypeSelect value={tkrType} onChange={setTkrType} />
      <Button
        size="sm"
        disabled={!value.trim()}
        onClick={() => {
          props.onAdd(value.trim(), tkrType);
          setValue("");
        }}
      >
        {props.buttonLabel}
      </Button>
    </div>
  );
};

/** Literal value + TKR[...] type, used for const nodes. */
const ConstInlineAdd = (props: {
  onAdd: (value: string, tkrType: string) => void;
}) => {
  const [value, setValue] = useState("");
  const [tkrType, setTkrType] = useState<string>(TKR_TYPES[0]);
  return (
    <div className="flex gap-2">
      <Input
        placeholder="const value, e.g. 42"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        className="h-8"
      />
      <TypeSelect value={tkrType} onChange={setTkrType} />
      <Button
        size="sm"
        disabled={!value.trim()}
        onClick={() => {
          props.onAdd(value.trim(), tkrType);
          setValue("");
        }}
      >
        Add const
      </Button>
    </div>
  );
};

/** One existing Workflow that can be referenced as an eval/loop/map node.
 * Its real input/output port names are fetched so the added node has the
 * right number of handles instead of a generic placeholder. */
const WorkflowRefRow = (props: {
  workflow: WorkflowDisplay;
  onAdd: (data: BuilderNodeData) => void;
}) => {
  const { workflow } = props;
  const { data: ports } = useWorkflowPortsQuery(workflow.id);

  return (
    <div className="flex items-center justify-between gap-2 border rounded-md px-2 py-1">
      <Link
        to="/workflows/$wid/nodes/$loc"
        params={{ wid: workflow.id, loc: "-" }}
        search={{ openEvals: [], openLoops: [], openMaps: [] }}
        className="text-sm truncate hover:underline"
      >
        {workflow.name ?? workflow.id}
      </Link>
      <div className="flex gap-1 shrink-0">
        {(["eval", "loop", "map"] as const).map((kind) => (
          <Button
            key={kind}
            size="sm"
            variant="outline"
            disabled={!ports}
            onClick={() =>
              props.onAdd({
                kind,
                label: `${kind}_${workflow.name ?? workflow.id}`,
                inputs: ports?.inputs ?? [],
                outputs: ports?.outputs ?? [],
                refWorkflowId: workflow.id,
                refWorkflowName: workflow.name ?? workflow.id,
              })
            }
          >
            {kind}
          </Button>
        ))}
      </div>
    </div>
  );
};

/** Drawer of things that can be dropped onto the canvas: real port-level
 * primitives (input/output/const), a fake catalogue of Worker functions
 * (since no real Worker registry exists yet), and links to existing
 * Workflows for eval/loop/map subgraph references. */
export const ComponentDrawer = (props: {
  onAdd: (data: BuilderNodeData) => void;
}) => {
  const { data: workflows } = useWorkflowsQuery();

  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button variant="outline" size="sm">
          <Blocks /> Components
        </Button>
      </SheetTrigger>
      <SheetContent side="left" className="overflow-y-auto p-4 gap-6">
        <SheetHeader className="p-0">
          <SheetTitle>Add to graph</SheetTitle>
        </SheetHeader>

        <Section title="Ports">
          <PortInlineAdd
            placeholder="input name"
            buttonLabel="Add input"
            onAdd={(name, tkrType) =>
              props.onAdd({
                kind: "input",
                label: name,
                inputs: [],
                outputs: [name],
                tkrType,
              })
            }
          />
          <PortInlineAdd
            placeholder="output name"
            buttonLabel="Add output"
            onAdd={(name, tkrType) =>
              props.onAdd({
                kind: "output",
                label: name,
                inputs: ["value"],
                outputs: [],
                tkrType,
              })
            }
          />
          <ConstInlineAdd
            onAdd={(value, tkrType) =>
              props.onAdd({
                kind: "const",
                label: `const_${value}`,
                inputs: [],
                outputs: ["value"],
                constValue: value,
                tkrType,
              })
            }
          />
        </Section>

        <Section title="Control flow">
          <Button
            variant="secondary"
            size="sm"
            onClick={() =>
              props.onAdd({
                kind: "ifelse",
                label: "if_else",
                inputs: ["cond", "if_true", "if_false"],
                outputs: ["value"],
              })
            }
          >
            Add if/else
          </Button>
        </Section>

        <Section title="Worker functions (mock catalogue)">
          <div className="flex flex-col gap-3">
            {FAKE_WORKERS.map((worker) => (
              <div key={worker.worker} className="flex flex-col gap-1">
                <div className="text-xs font-mono text-muted-foreground">
                  {worker.worker}
                </div>
                {worker.tasks.map((task) => (
                  <div
                    key={task.name}
                    className="flex items-center justify-between gap-2 border rounded-md px-2 py-1"
                  >
                    <div className="text-sm font-mono truncate">
                      {task.name}({task.inputs.join(", ")})
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() =>
                        props.onAdd({
                          kind: "task",
                          label: task.name,
                          inputs: task.inputs,
                          outputs: task.outputs,
                          workerName: worker.worker,
                          taskName: task.name,
                        })
                      }
                    >
                      Add
                    </Button>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </Section>

        <Section title="Eval / loop / map an existing Workflow">
          <div className="flex flex-col gap-2">
            {(workflows ?? []).length === 0 && (
              <div className="text-sm text-muted-foreground">
                No existing Workflows to reference yet.
              </div>
            )}
            {(workflows ?? []).map((workflow) => (
              <WorkflowRefRow
                key={workflow.id}
                workflow={workflow}
                onAdd={props.onAdd}
              />
            ))}
          </div>
        </Section>
      </SheetContent>
    </Sheet>
  );
};
