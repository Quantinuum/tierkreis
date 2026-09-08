use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort, PortIndex as _};
use hugr_core::hugr::internal::PortgraphNodeMap;
use petgraph::algo::dominators::{self, Dominators};
use portgraph::NodeIndex;

use super::{GraphWithFuncs, convert_dfg};
use crate::graph::{NodeDefinition, WorkflowGraph};

struct DomTreeNode<N> {
    node: N,
    // In topsort order (child before any sibling it can reach)
    children: Vec<(GatingPath<N>, DomTreeNode<N>)>,
    // TODO should both the following be a single GatingPath each? (Losing the cache)
    exit_edges: Vec<(GatingPath<N>, N)>, // Include cache of target node
    loop_backedges: Vec<GatingPath<N>>,
}

impl<N: HugrNode> DomTreeNode<N> {
    fn build_graph(
        &self,
        graph: &mut GraphWithFuncs<N>,
        hugr: &impl HugrView<Node = N>,
        this_block_inputs: Vec<(NodeIndex, String)>,
    ) -> miette::Result<(
        Option<Vec<(NodeIndex, String)>>, // any values delivered to exit node (only if in this DomTreeNode)
        HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>, // values delivered to exit edges of this DomTreeNode
    )> {
        if !self.loop_backedges.is_empty() {
            let mut blocks = HashSet::new();
            let mut queue = VecDeque::from_iter(
                self.loop_backedges
                    .iter()
                    .flat_map(GatingPath::leaves)
                    .map(|(n, _p)| n),
            );
            while let Some(n) = queue.pop_front() {
                if n == self.node || !blocks.insert(n) {
                    continue;
                }
                queue.extend(hugr.input_neighbours(n));
            }
            let loop_exits = blocks
                .iter()
                .flat_map(|&n| hugr.node_outputs(n).map(move |p| (n, p)))
                .filter(|(n, p)| !blocks.contains(&hugr.single_linked_input(*n, *p).unwrap().0))
                .collect::<Vec<_>>();
            let Ok((loop_exit_block, p)) = loop_exits.into_iter().exactly_one() else {
                // Need to build a single exit that collects all these together, with a sum type or similar
                // that identifies which exit was taken and where to go next
                todo!("Multi-exit loops")
            };
            let loop_exit_tys = hugr
                .get_optype(loop_exit_block)
                .as_dataflow_block()
                .unwrap()
                .successor_input(p.index())
                .unwrap();
            let loop_in_tys = &hugr
                .get_optype(self.node)
                .as_dataflow_block()
                .unwrap()
                .inputs;
            assert_eq!(&loop_exit_tys, loop_in_tys); // TODO: allow exitting with only a subset? But, how to identify?
            todo!("Single-exit loop")
            // 1. compile the repeat value as the value returned to the header.
            //   This suggests all backedges should be unified into a single GatingPath
            // 2. compute the GatingPath of getting to the unique exit edge
            //   by traversing DomTree from the edge source back up to the header
            //   and using GatingPath::concat
            // 3. the loop body outputs are
            //   1. the predicate, did we get to that exit edge (inverted)
            //   2. the values, selected (via a UNION of the loop-backedge GatingPath and the exit-edge GatingPath)
            //      from the usual block_outputs as below
            // ....all that within a new GraphWithFuncs, that we then insert as a constant and push through a NodeDefinition::Loop
        }
        let Some(bb) = hugr.get_optype(self.node).as_dataflow_block() else {
            assert!(hugr.get_optype(self.node).is_exit_block());
            assert!(self.exit_edges.is_empty());
            assert!(self.children.is_empty());
            return Ok((Some(this_block_inputs), HashMap::new()));
        };
        let mut block_outputs: HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>> =
            HashMap::new();
        let mut block_preds: HashMap<N, (NodeIndex, String)> = HashMap::new();

        // Compile body. (Easy - the complexity of this function is all about the branches!)
        let this_block_outs = convert_dfg(hugr, self.node, graph, this_block_inputs)?;

        // Edges from "root"
        assert_eq!(hugr.node_outputs(self.node).count(), bb.sum_rows.len());
        // Guppy generates only unit sum branch predicates
        assert!(bb.sum_rows.iter().all(|row| row.is_empty()));
        assert!([1, 2].contains(&bb.sum_rows.len())); // For now we support only two-way branches
        if bb.sum_rows.len() == 2 {
            block_preds.insert(self.node, this_block_outs[0].clone());
        }
        // Do not include predicate (the first output) in the block outputs - any predicate
        // necessary will be built by the relevant GatingPath reading from block_preds
        for p in hugr.node_outputs(self.node) {
            block_outputs.insert((self.node, p), this_block_outs[1..].to_vec());
        }

        let mut exit_node_outs = None;

        for (child_path, child) in &self.children {
            let child_inputs =
                child_path.build_inputs(&mut graph.graph, &block_outputs, &block_preds);
            let (exit_block_outs, exit_edge_outs) = child.build_graph(graph, hugr, child_inputs)?;
            if let Some(exit_block_outs) = exit_block_outs {
                let prev = exit_node_outs.replace(exit_block_outs);
                assert!(prev.is_none())
            }
            block_outputs.extend(exit_edge_outs);
        }
        Ok((
            exit_node_outs,
            self.exit_edges
                .iter()
                .flat_map(|(gp, _tgt)| gp.leaves())
                .map(|np| (np, block_outputs.remove(&np).unwrap()))
                .collect(),
        ))
    }
}

fn build_dom_tree<H: HugrView>(hugr: &H, cfg: H::Node) -> DomTreeNode<H::Node> {
    fn build<H: HugrView>(
        hugr: &H,
        doms: &Dominators<NodeIndex>,
        n: H::Node,
        node_map: &H::RegionPortgraphNodes,
    ) -> DomTreeNode<H::Node> {
        let ni = node_map.to_portgraph(n);
        let children_by_bb = doms
            .immediately_dominated_by(ni)
            .map(|c| build(hugr, doms, node_map.from_portgraph(c), node_map))
            .map(|c| (c.node, c))
            .collect::<HashMap<_, _>>();
        let mut exit_edges = Vec::new();
        let mut loop_backedges = Vec::new();

        let mut child_paths = HashMap::<H::Node, GatingPath<H::Node>>::new();
        // Process edges from this node (perhaps a loop header)
        let path = hugr
            .node_outputs(n.into())
            .exactly_one()
            .ok()
            .map(|p| GatingPath::Always(n, p));
        for outport in hugr.node_outputs(n.into()) {
            let path = path.clone().unwrap_or(GatingPath::branch(n, outport));
            // Control Flow outports should have exactly one outgoing edge
            let (tgt, _) = hugr
                .linked_inputs(n.into(), outport)
                .exactly_one()
                .ok()
                .unwrap();
            if children_by_bb.contains_key(&tgt) {
                child_paths.entry(tgt).or_default().union(&path);
            } else if tgt == n {
                loop_backedges.push(path);
            } else {
                exit_edges.push((path, tgt));
            }
        }
        // We want to process children in reverse topsort order: any child C1 with an exit edge to C2, must be processed *before* C2.
        let mut ordered_children = Vec::new();
        fn rev_sort<N: HugrNode>(
            ordered: &mut Vec<N>,
            child: N,
            remaining_children: &mut HashMap<N, &DomTreeNode<N>>,
        ) {
            let Some(dtn) = remaining_children.remove(&child) else {
                return;
            };
            // Targets of exit edges pushed onto <ordered> first
            for (_, succ) in &dtn.exit_edges {
                rev_sort(ordered, *succ, remaining_children);
            }
            ordered.push(child);
        }
        let mut remaining_children = children_by_bb.iter().map(|(n, c)| (*n, c)).collect();
        for child in children_by_bb.values() {
            rev_sort(&mut ordered_children, child.node, &mut remaining_children);
        }
        // This means targets of exit edges will be processed *after* all the sources of said exit edges:
        ordered_children.reverse();

        // Now process children in the determined order
        let mut children = Vec::new();
        let mut children_by_bb = children_by_bb;
        for child in ordered_children {
            let path_to_child = child_paths.remove(&child).unwrap();
            let child_dtn = children_by_bb.remove(&child).unwrap();
            for (path_from_child_to_exit, dst) in &child_dtn.exit_edges {
                let path_to_exit = path_to_child.concat(path_from_child_to_exit);
                assert!(
                    // if dst has no dominator, dst is the entry node
                    doms.immediate_dominator(node_map.to_portgraph(*dst))
                        .is_none_or(|tgt_dom|
                        //otherwise, tgt_dom must be ni or some dominator thereof
                        // (i.e. tgt is a sibling of an nonstrict-ancestor of ni).
                    doms.dominators(ni).unwrap().contains(&tgt_dom))
                );
                if children_by_bb.contains_key(dst) {
                    child_paths.entry(*dst).or_default().union(&path_to_exit);
                } else if *dst == n {
                    loop_backedges.push(path_to_exit);
                } else {
                    exit_edges.push((path_to_exit, *dst));
                }
            }
            children.push((path_to_child, child_dtn))
        }
        DomTreeNode {
            node: n,
            children,
            exit_edges,
            loop_backedges,
        }
    }

    let (doms, node_map) = compute_dominators(hugr, cfg);
    let entry = hugr.children(cfg).next().unwrap();
    assert_eq!(doms.root(), node_map.to_portgraph(entry));
    build(&hugr, &doms, entry, &node_map)
}

fn compute_dominators<H: HugrView>(
    hugr: &H,
    parent: H::Node,
) -> (Dominators<NodeIndex>, H::RegionPortgraphNodes) {
    let sg = hugr.scheduling_graph(parent);
    let entry_node = hugr.children(parent).next().unwrap();
    let doms = dominators::simple_fast(sg.petgraph(), sg.node_to_pg(entry_node));
    (doms, sg.into_node_map())
}

pub(super) fn convert_cfg<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut GraphWithFuncs<H::Node>,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    if !hugr.get_optype(node).is_cfg() {
        return Err(miette::miette!(
            "Node {:?} is not a CFG node, but has OpType {:?}",
            node,
            hugr.get_optype(node)
        ));
    }
    let tr = build_dom_tree(hugr, node);
    let (outs, _) = tr.build_graph(graph, hugr, inputs)?;
    Ok(outs.unwrap())
}

#[derive(Clone, Debug, Default)]
enum GatingPath<N> {
    // Contains the edge along which control flow will arrive
    Always(N, OutgoingPort),
    #[default]
    Never,
    // Invariant: HashMap always non-empty
    Branch(N, HashMap<OutgoingPort, GatingPath<N>>),
}

impl<N: HugrNode> GatingPath<N> {
    fn branch(node: N, port: OutgoingPort) -> Self {
        GatingPath::Branch(
            node,
            HashMap::from([(port, GatingPath::Always(node, port))]),
        )
    }

    fn build_inputs(
        &self,
        graph: &mut WorkflowGraph,
        block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
        block_preds: &HashMap<N, (NodeIndex, String)>,
    ) -> Vec<(NodeIndex, String)> {
        match self {
            GatingPath::Never => panic!("Cannot build inputs for GatingPath::Never"),
            GatingPath::Always(n, p) => block_outputs.get(&(*n, *p)).unwrap().clone(),
            GatingPath::Branch(br, opts) => {
                assert!([1, 2].contains(&opts.len())); // guppy only produces bools
                if let Ok(path) = opts.values().exactly_one() {
                    return path.build_inputs(graph, block_outputs, block_preds);
                }
                let fal =
                    opts.get(&0.into())
                        .unwrap()
                        .build_inputs(graph, block_outputs, block_preds);
                let tr =
                    opts.get(&1.into())
                        .unwrap()
                        .build_inputs(graph, block_outputs, block_preds);
                let pred = block_preds.get(br).unwrap();
                fal.iter()
                    .zip_eq(&tr)
                    .map(|(f, t)| {
                        if f == t {
                            t.clone()
                        } else {
                            // crate::builder::if_else is test-only, duplicating it here
                            let node = graph.add_node(
                                NodeDefinition::IfElse {},
                                [
                                    "pred".to_string(),
                                    "if_true".to_string(),
                                    "if_false".to_string(),
                                ],
                                ["value".to_string()],
                            );
                            graph
                                .link_nodes_by_port_name(pred.0, &pred.1, node, "pred")
                                .unwrap();
                            graph
                                .link_nodes_by_port_name(f.0, &f.1, node, "if_false")
                                .unwrap();
                            graph
                                .link_nodes_by_port_name(t.0, &t.1, node, "if_true")
                                .unwrap();
                            (node, "value".into())
                        }
                    })
                    .collect()
            }
        }
    }

    fn concat(&self, other: &GatingPath<N>) -> Self {
        match self {
            GatingPath::Never => panic!("Cannot concatenate with Never"), // Or return Never?
            GatingPath::Always(_, _) => other.clone(),
            GatingPath::Branch(node, map) => GatingPath::Branch(
                *node,
                map.into_iter()
                    .map(|(k, v)| (*k, v.concat(other)))
                    .collect(),
            ),
        }
    }

    fn leaves(&self) -> Vec<(N, OutgoingPort)> {
        match self {
            GatingPath::Never => vec![],
            GatingPath::Always(node, port) => vec![(*node, port.clone())],
            GatingPath::Branch(_, map) => map.values().flat_map(|v| v.leaves()).collect(),
        }
    }

    fn union(&mut self, other: &GatingPath<N>) {
        if matches!(other, GatingPath::Never) {
            return;
        }
        match self {
            GatingPath::Never => {
                *self = other.clone();
            }
            GatingPath::Always(_, _) => {
                panic!("Union of Always with {other:?}");
            }
            GatingPath::Branch(n, map) => {
                if let GatingPath::Branch(n2, map2) = other
                    && n == n2
                {
                    for (k, v) in map2 {
                        map.entry(*k).or_default().union(v)
                    }
                } else {
                    panic!("Cannot union Branch({n:?}) with {other:?}");
                }
            }
        }
    }
}
