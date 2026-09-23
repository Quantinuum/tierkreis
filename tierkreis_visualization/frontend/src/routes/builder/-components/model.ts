/**
 * Data model + fake catalogue + code generation for the graph builder POC.
 * Node/edge state lives entirely client-side; nothing here talks to the backend
 * except linking to real existing Workflows for eval/loop/map references.
 */
import type { Edge, Node } from "@xyflow/react";

export type BuilderNodeKind =
  | "input"
  | "output"
  | "const"
  | "task"
  | "eval"
  | "loop"
  | "map"
  | "ifelse";

export type BuilderNodeData = {
  kind: BuilderNodeKind;
  label: string;
  inputs: string[];
  outputs: string[];
  workerName?: string;
  taskName?: string;
  constValue?: string;
  // The TKR[...] element type for input/output/const nodes, e.g. "int" or "list[str]".
  tkrType?: string;
  refWorkflowId?: string;
  refWorkflowName?: string;
  onDelete?: () => void;
  [key: string]: unknown;
};

export type BuilderNode = Node<BuilderNodeData>;
export type BuilderEdge = Edge;

// The TKR[...] element types offered when adding an input/output/const node.
export const TKR_TYPES = [
  "int",
  "float",
  "str",
  "bool",
  "complex",
  "list[int]",
  "list[float]",
  "list[str]",
  "list[bool]",
  "list[complex]",
] as const;

export const inHandle = (port: string) => `in:${port}`;
export const outHandle = (port: string) => `out:${port}`;
export const portFromHandle = (handle: string) =>
  handle.replace(/^(in|out):/, "");

// --- Fake worker catalogue, used to populate the component drawer -------

export type FakeTask = { name: string; inputs: string[]; outputs: string[] };
export type FakeWorker = { worker: string; tasks: FakeTask[] };

export const FAKE_WORKERS: FakeWorker[] = [
  {
    worker: "builtins",
    tasks: [
      { name: "iadd", inputs: ["a", "b"], outputs: ["value"] },
      { name: "itimes", inputs: ["a", "b"], outputs: ["value"] },
      { name: "iminus", inputs: ["a", "b"], outputs: ["value"] },
      { name: "not", inputs: ["value"], outputs: ["value"] },
    ],
  },
  {
    worker: "quantum_worker",
    tasks: [
      { name: "h", inputs: ["qubit"], outputs: ["qubit"] },
      {
        name: "cx",
        inputs: ["control", "target"],
        outputs: ["control", "target"],
      },
      { name: "measure", inputs: ["qubit"], outputs: ["bit"] },
    ],
  },
  {
    worker: "hpc_worker",
    tasks: [
      {
        name: "simulate_circuit",
        inputs: ["circuit", "shots"],
        outputs: ["counts"],
      },
      { name: "optimize", inputs: ["circuit"], outputs: ["circuit"] },
    ],
  },
];

let nextId = 1;
export const newNodeId = () => `n${nextId++}`;

const toIdentifier = (label: string): string => {
  const cleaned = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "_");
  const safe = cleaned.replace(/^_+|_+$/g, "") || "node";
  return /^[0-9]/.test(safe) ? `_${safe}` : safe;
};

/** Topologically sort nodes by their edge dependencies; ties broken by insertion order. */
const topoSort = (
  nodes: BuilderNode[],
  edges: BuilderEdge[],
): BuilderNode[] => {
  const indexOf = new Map(nodes.map((n, i) => [n.id, i]));
  const indegree = new Map(nodes.map((n) => [n.id, 0]));
  const dependents = new Map<string, string[]>(nodes.map((n) => [n.id, []]));
  for (const edge of edges) {
    if (!indegree.has(edge.target) || !dependents.has(edge.source)) continue;
    indegree.set(edge.target, (indegree.get(edge.target) ?? 0) + 1);
    dependents.get(edge.source)?.push(edge.target);
  }

  const ready = nodes.filter((n) => (indegree.get(n.id) ?? 0) === 0);
  ready.sort((a, b) => (indexOf.get(a.id) ?? 0) - (indexOf.get(b.id) ?? 0));
  const order: BuilderNode[] = [];
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const queue = [...ready];
  const seen = new Set<string>();
  while (queue.length > 0) {
    const node = queue.shift();
    if (!node || seen.has(node.id)) continue;
    seen.add(node.id);
    order.push(node);
    for (const depId of dependents.get(node.id) ?? []) {
      const remaining = (indegree.get(depId) ?? 1) - 1;
      indegree.set(depId, remaining);
      if (remaining === 0) {
        const dep = byId.get(depId);
        if (dep) queue.push(dep);
      }
    }
    queue.sort((a, b) => (indexOf.get(a.id) ?? 0) - (indexOf.get(b.id) ?? 0));
  }
  // Anything left over is part of a cycle (or disconnected weirdly) - append
  // in original order so generation never silently drops a node.
  for (const node of nodes) {
    if (!seen.has(node.id)) order.push(node);
  }
  return order;
};

const toPascalCase = (label: string): string =>
  toIdentifier(label)
    .split("_")
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join("") || "Node";

const literalFor = (raw: string, tkrType?: string): string => {
  const trimmed = raw.trim();
  if (tkrType === "bool")
    return trimmed.toLowerCase() === "true" ? "True" : "False";
  if (tkrType === "str") return JSON.stringify(raw);
  if (tkrType?.startsWith("list")) return trimmed || "[]";
  if (tkrType === "complex") return trimmed || "0j";
  if (tkrType === "int" || tkrType === "float") return trimmed || "0";
  // No type selected: best-effort auto-detect between a number and a string.
  return Number.isFinite(Number(trimmed)) && trimmed !== ""
    ? trimmed
    : JSON.stringify(raw);
};

/**
 * Render the current graph using the same typed builder syntax as the real
 * Workflow definitions (see tests/controller/typed_graphdata.py): `g.inputs`
 * for reading inputs, `g.finish_with_outputs(...)` for outputs, and bare
 * task calls like `iadd(a=x, b=y)` wrapped in `g.task(...)`.
 *
 * One deliberate exception: eval/loop/map nodes reference a Workflow that
 * isn't locally importable in this POC, so the graph-returning function call
 * is replaced with a `workflow_ref("name")` placeholder - everything else
 * (the typed input model, `g.eval`/`g.loop`/`g.map`) still follows the real
 * syntax.
 */
export const generateCode = (
  nodes: BuilderNode[],
  edges: BuilderEdge[],
): string => {
  if (nodes.length === 0) {
    return "# Add nodes from the component drawer to get started.";
  }

  const inputNodes = nodes.filter((n) => n.data.kind === "input");
  const outputNodes = nodes.filter((n) => n.data.kind === "output");
  const bodyNodes = nodes.filter(
    (n) => n.data.kind !== "input" && n.data.kind !== "output",
  );
  const orderedBody = topoSort(bodyNodes, edges);

  const varNames = new Map<string, string>();
  const used = new Set<string>();
  for (const node of bodyNodes) {
    let name = toIdentifier(node.data.label);
    let suffix = 2;
    while (used.has(name)) {
      name = `${toIdentifier(node.data.label)}_${suffix}`;
      suffix += 1;
    }
    used.add(name);
    varNames.set(node.id, name);
  }

  const inputField = (node: BuilderNode) => toIdentifier(node.data.label);
  const outputField = (node: BuilderNode) => toIdentifier(node.data.label);

  // Inputs are always wrapped in a GraphInputs NamedTuple (even a single
  // field), so reading one is always `g.inputs.<field>`.
  const inputExpr = (node: BuilderNode) => `g.inputs.${inputField(node)}`;

  // A node's own result: bare `var` when it has a single output port
  // (the common case), `var.<port>` when it has several.
  const resultExpr = (node: BuilderNode, port: string) =>
    node.data.outputs.length <= 1
      ? varNames.get(node.id)
      : `${varNames.get(node.id)}.${port}`;

  const sourceFor = (nodeId: string, port: string): string => {
    const edge = edges.find(
      (e) =>
        e.target === nodeId && portFromHandle(e.targetHandle ?? "") === port,
    );
    if (!edge) return "None  # TODO: connect an input";
    const sourceNode =
      inputNodes.find((n) => n.id === edge.source) ??
      bodyNodes.find((n) => n.id === edge.source);
    if (!sourceNode) return "None  # TODO: connect an input";
    const sourcePort = portFromHandle(edge.sourceHandle ?? "");
    return sourceNode.data.kind === "input"
      ? inputExpr(sourceNode)
      : (resultExpr(sourceNode, sourcePort) ?? "None");
  };

  // Build a typed positional argument for a Node's declared input ports:
  // omitted if none, bare if one, else a placeholder NamedTuple call.
  const argsExprFor = (node: BuilderNode, className: string): string => {
    const values = node.data.inputs.map((port) => sourceFor(node.id, port));
    if (values.length === 0) return "";
    if (values.length === 1) return `, ${values[0]}`;
    const kwargs = node.data.inputs
      .map((port, i) => `${port}=${values[i]}`)
      .join(", ");
    return `, ${className}(${kwargs})`;
  };

  const usedTasks = new Map<string, Set<string>>();
  for (const node of bodyNodes) {
    if (node.data.kind !== "task" || !node.data.workerName) continue;
    const set = usedTasks.get(node.data.workerName) ?? new Set<string>();
    if (node.data.taskName) set.add(node.data.taskName);
    usedTasks.set(node.data.workerName, set);
  }

  const importLines: string[] = [];
  for (const worker of [...usedTasks.keys()].sort((a, b) =>
    a === "builtins" ? -1 : a.localeCompare(b),
  )) {
    const tasks = [...(usedTasks.get(worker) ?? [])].sort();
    const module =
      worker === "builtins" ? "tierkreis.builtins" : `workers.${worker}`;
    importLines.push(`from ${module} import ${tasks.join(", ")}`);
  }

  // Inputs/outputs are always wrapped in a NamedTuple, even for a single
  // field, so the types line up with the actual builder API's `g.inputs.<field>`
  // access pattern regardless of how many ports there are.
  const needsNamedTuple = inputNodes.length > 0 || outputNodes.length > 0;
  const needsEmptyModel = inputNodes.length === 0 || outputNodes.length === 0;
  const needsAny =
    inputNodes.some((n) => !n.data.tkrType) ||
    outputNodes.some((n) => !n.data.tkrType);

  const inputsType = inputNodes.length === 0 ? "EmptyModel" : "GraphInputs";
  const outputsType = outputNodes.length === 0 ? "EmptyModel" : "GraphOutputs";

  const modelsImports = [
    "Workflow",
    ...(needsNamedTuple ? ["TKR"] : []),
    ...(needsEmptyModel ? ["EmptyModel"] : []),
  ];

  const lines: string[] = [
    "# Auto-generated preview from the graph builder.",
    "# Editing this code does not (yet) feed back into the graph - POC only.",
    ...(needsAny ? ["from typing import Any"] : []),
    ...(needsNamedTuple ? ["from typing import NamedTuple"] : []),
    "from tierkreis.builder import Graph",
    ...importLines,
    `from tierkreis.models import ${modelsImports.join(", ")}`,
    "",
  ];

  if (inputNodes.length > 0) {
    lines.push("class GraphInputs(NamedTuple):");
    for (const node of inputNodes) {
      lines.push(`    ${inputField(node)}: TKR[${node.data.tkrType || "Any"}]`);
    }
    lines.push("");
  }
  if (outputNodes.length > 0) {
    lines.push("class GraphOutputs(NamedTuple):");
    for (const node of outputNodes) {
      lines.push(
        `    ${outputField(node)}: TKR[${node.data.tkrType || "Any"}]`,
      );
    }
    lines.push("");
  }

  lines.push(`def build_graph() -> Workflow[${inputsType}, ${outputsType}]:`);
  lines.push(`    g = Graph(${inputsType}, ${outputsType})`);
  lines.push("");

  for (const node of orderedBody) {
    const v = varNames.get(node.id);
    const { data } = node;
    if (data.kind === "const") {
      lines.push(
        `    ${v} = g.const(${literalFor(data.constValue ?? "None", data.tkrType)})`,
      );
    } else if (data.kind === "task") {
      const args = data.inputs
        .map((port) => `${port}=${sourceFor(node.id, port)}`)
        .join(", ");
      lines.push(`    ${v} = g.task(${data.taskName}(${args}))`);
    } else if (
      data.kind === "eval" ||
      data.kind === "loop" ||
      data.kind === "map"
    ) {
      const refName = data.refWorkflowName ?? "unknown";
      const className = `${toPascalCase(refName)}Input`;
      const placeholder = `workflow_ref("${refName}")`;
      lines.push(
        `    ${v} = g.${data.kind}(${placeholder}${argsExprFor(node, className)})  # "${refName}" isn't locally importable in this POC`,
      );
    } else if (data.kind === "ifelse") {
      const cond = sourceFor(node.id, "cond");
      const ifTrue = sourceFor(node.id, "if_true");
      const ifFalse = sourceFor(node.id, "if_false");
      lines.push(`    ${v} = g.ifelse(${cond}, ${ifTrue}, ${ifFalse})`);
    }
  }

  lines.push("");
  if (outputNodes.length === 0) {
    lines.push("    return g.finish_with_outputs(EmptyModel())");
  } else {
    const kwargs = outputNodes
      .map(
        (node) =>
          `${outputField(node)}=${sourceFor(node.id, node.data.inputs[0] ?? "value")}`,
      )
      .join(", ");
    lines.push(`    return g.finish_with_outputs(GraphOutputs(${kwargs}))`);
  }

  return lines.join("\n");
};
