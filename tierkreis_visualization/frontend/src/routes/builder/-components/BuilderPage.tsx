import { Button } from "@/components/ui/button";
import {
  addEdge,
  Background,
  Controls,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
  type Connection,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useCallback, useMemo, useRef } from "react";
import { RotateCcw } from "lucide-react";
import { ComponentDrawer } from "./ComponentDrawer";
import { CodeDrawer } from "./CodeDrawer";
import { BuilderNodeView } from "./BuilderNodeView";
import {
  generateCode,
  newNodeId,
  type BuilderEdge,
  type BuilderNode,
  type BuilderNodeData,
} from "./model";

const nodeTypes = { builder: BuilderNodeView };

let addCount = 0;

export const BuilderPage = () => {
  const [nodes, setNodes, onNodesChange] = useNodesState<BuilderNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<BuilderEdge>([]);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const { screenToFlowPosition } = useReactFlow();

  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges],
  );

  const deleteNode = useCallback(
    (id: string) => {
      setNodes((nds) => nds.filter((n) => n.id !== id));
      setEdges((eds) => eds.filter((e) => e.source !== id && e.target !== id));
    },
    [setNodes, setEdges],
  );

  const addNode = useCallback(
    (data: BuilderNodeData) => {
      addCount += 1;
      // Place near the centre of the currently visible canvas (accounting for
      // pan/zoom), cascading slightly so repeated adds don't fully overlap.
      const bounds = wrapperRef.current?.getBoundingClientRect();
      const cascade = (addCount % 8) * 30;
      const screenX =
        (bounds ? bounds.left + bounds.width / 2 : 0) - 100 + cascade;
      const screenY =
        (bounds ? bounds.top + bounds.height / 2 : 0) - 80 + cascade;
      const position = screenToFlowPosition({ x: screenX, y: screenY });
      const id = newNodeId();
      const node: BuilderNode = {
        id,
        type: "builder",
        position,
        data: { ...data, onDelete: () => deleteNode(id) },
      };
      setNodes((nds) => [...nds, node]);
    },
    [setNodes, screenToFlowPosition, deleteNode],
  );

  const reset = useCallback(() => {
    setNodes([]);
    setEdges([]);
  }, [setNodes, setEdges]);

  const code = useMemo(() => generateCode(nodes, edges), [nodes, edges]);

  return (
    <div ref={wrapperRef} className="relative h-full w-full">
      <div className="absolute top-2 left-2 z-20 flex items-center gap-2">
        <ComponentDrawer onAdd={addNode} />
        <CodeDrawer code={code} />
        <Button variant="outline" size="sm" onClick={reset}>
          <RotateCcw /> Reset
        </Button>
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        deleteKeyCode={["Backspace", "Delete"]}
        fitView
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
};
