import { CSSProperties } from "react";
import { calculateNodePositions } from "@/graph/parseGraph";
import { AppNode, BackendNode } from "@/nodes/types";
import { nodeHeight, nodeWidth } from "@/data/constants";
import { Edge } from "@xyflow/react";
import { loc_depth, loc_steps } from "@/data/loc";

interface ShallowNode {
  id: string;
  style?: CSSProperties;
  position: { x: number; y: number };
  parentId?: string;
}

export function bottomUpLayout(nodes: BackendNode[], edges: Edge[]) {
  // sort nodes by levels i.e. number of : in their id
  // calculate each level individually
  const nodeLevels = new Map<number, ShallowNode[]>();
  nodes.forEach((node) => {
    const level = loc_depth(node.id) - 1;
    if (!nodeLevels.has(level)) {
      nodeLevels.set(level, []);
    }
    nodeLevels.get(level)?.push({
      id: node.id,
      style: node.style,
      position: node.position,
      parentId: node.parentId,
    });
  });
  const newNodes: AppNode[] = [];
  let previousNodes: ShallowNode[] = [];
  // construct graph from most nested level
  const levelKeys = Array.from(nodeLevels.keys()).sort((a, b) => b - a);
  for (const level of levelKeys) {
    const padding = level == 0 ? 1 : 20;
    const currentNodes = nodeLevels.get(level);
    if (!currentNodes) continue;

    const idsInLevel = new Set(currentNodes.map((nodeInfo) => nodeInfo.id));
    const levelNodes = nodes.filter((node) => idsInLevel.has(node.id));
    const oldEdges = restoreEdges(level, edges);
    resizeGroupNodesToFitChildren(levelNodes, previousNodes, 20);
    // if we don't do this groups wise dagre puts them all on the same plane
    const groups = levelNodes.reduce<{ [key: string]: BackendNode[] }>(
      (acc, currentNode) => {
        const key = currentNode.parentId ? currentNode.parentId : "-";
        if (!acc[key]) {
          acc[key] = [];
        }
        acc[key].push(currentNode);
        return acc;
      },
      {}
    );
    const data: ReturnType<typeof calculateNodePositions> = [];
    for (const group of Object.values(groups)) {
      const tmp = calculateNodePositions(
        group,
        [...edges, ...oldEdges],
        padding
      );
      data.push(...tmp);
    }
    const tmpNodes = levelNodes.map((node) => ({
      ...node,
      position:
        data.find((position) => position.id === node.id) || node.position,
    }));
    newNodes.push(...tmpNodes);
    previousNodes = tmpNodes;
  }

  return newNodes.reverse();
}

function resizeGroupNodesToFitChildren(
  nodesToResize: ShallowNode[],
  childNodes: ShallowNode[],
  padding: number
) {
  // resizes all the nodes in nodesToResize to fit their children
  if (!childNodes.length) return;
  // only groups have children
  for (const node of nodesToResize) {
    const children = childNodes.filter((child) => child.parentId === node.id);
    if (!children.length) continue;
    const xPositions = children.map(
      (child) => child.position.x + Number(child.style?.width ?? nodeWidth)
    );
    const yPositions = children.map(
      (child) => child.position.y + Number(child.style?.height ?? nodeHeight)
    );
    node.style = {
      width: Math.max(...xPositions) + padding,
      height: Math.max(...yPositions) + padding,
    };
  }
}

function restoreEdges(level: number, edges: Edge[]) {
  const levelEdges = edges.filter(
    (edge) =>
      loc_depth(edge.source) <= level + 1 || loc_depth(edge.target) <= level + 1
  );
  const newEdges = new Set<Edge>();
  for (const edge of levelEdges) {
    const source = loc_steps(edge.source);
    const target = loc_steps(edge.target);
    if (source.length === target.length) continue; // don't need to update
    if (source.length <= level + 1) {
      newEdges.add({
        ...edge,
        target: target.slice(0, level + 1).join("."),
      });
    } else {
      newEdges.add({
        ...edge,
        source: source.slice(0, level + 1).join("."),
      });
    }
  }
  return newEdges;
}
