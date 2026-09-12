use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter;

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort, PortIndex as _};
use hugr_core::hugr::internal::PortgraphNodeMap;
use petgraph::algo::dominators::{self, Dominators};
use portgraph::NodeIndex;

use super::{GraphWithFuncs, convert_dfg, graph_const, wire_up};
use crate::graph::{NodeDefinition, WorkflowGraph};

struct DomTreeNode<N> {
    node: N,
    // In topsort order (child before any sibling it can reach)
    children: Vec<(GatingPath<N>, DomTreeNode<N>)>,
    exit_edges: Option<GatingPath<N>>,
    loop_: Option<Loop<N>>,
}

// TODO: make exit_path into LeafPath, and put inside the Option (if None,
// the exit_path is the unique exit edge of the parent DomTreeNode).
struct Loop<N> {
    repeat_path: GatingPath<N>,
    exit_path: GatingPath<N>,
    post_loop: Option<Box<DomTreeNode<N>>>,
}

impl<N: HugrNode> DomTreeNode<N> {
    fn disconnect(&mut self, doms: &[N], outer: bool) -> (GatingPath<N>, DomTreeNode<N>) {
        for (child_idx, (child_path, child)) in self.children.iter_mut().enumerate() {
            if child.node == doms[0] {
                let (ep, dtn) = if doms.len() == 1 {
                    self.children.remove(child_idx)
                    // Note ep *is* child_path
                } else {
                    let (ep, dtn) = child.disconnect(&doms[1..], false);
                    (child_path.concat(&ep), dtn)
                };
                if !outer {
                    // Remove paths to leaves of the removed subtree, add a path to the root thereof
                    if let Some(ees) = dtn.exit_edges.as_ref() {
                        remove_opt(&mut self.exit_edges, &ep.clone().concat(ees));
                    }
                    union_opt(&mut self.exit_edges, &ep);
                }
                return (ep, dtn);
            }
        }
        if let Some(loop_) = self.loop_.as_mut()
            && let Some(post_loop) = loop_.post_loop.as_mut()
            && post_loop.node == doms[0]
        {
            panic!("Nested loop!"); // Just poison, don't think we're hitting this yet
            if doms.len() == 1 {
                return (loop_.exit_path.clone(), *loop_.post_loop.take().unwrap());
            }
            let (ep, dtn) = post_loop.disconnect(&doms[1..], false);
            return (loop_.exit_path.concat(&ep), dtn);
        }
        panic!("Node not found in children");
    }

    fn build_graph(
        &self,
        graph: &mut GraphWithFuncs<N>,
        hugr: &impl HugrView<Node = N>,
        this_block_inputs: Vec<(NodeIndex, String)>,
        block_preds: &mut HashMap<N, (NodeIndex, String)>,
    ) -> miette::Result<(
        Option<Vec<(NodeIndex, String)>>, // any values delivered to exit node (only if in this DomTreeNode)
        HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>, // values delivered to exit edges of this DomTreeNode
    )> {
        let Some(bb) = hugr.get_optype(self.node).as_dataflow_block() else {
            assert!(hugr.get_optype(self.node).is_exit_block());
            assert!(self.exit_edges.is_none());
            assert!(self.children.is_empty());
            return Ok((Some(this_block_inputs), HashMap::new()));
        };
        let Some(loop_) = self.loop_.as_ref() else {
            let (exit_node_vals, mut block_outputs) =
                self.build_nonloop(graph, hugr, bb, this_block_inputs, block_preds)?;
            return Ok((
                exit_node_vals,
                leaves(&self.exit_edges, hugr)
                    .into_iter()
                    .map(|lp| (lp.src, block_outputs.remove(&lp.src).unwrap()))
                    .collect(),
            ));
        };
        let outer_graph = graph; // Rename to disambiguate.

        let loop_in_tys = &hugr
            .get_optype(self.node)
            .as_dataflow_block()
            .unwrap()
            .inputs;
        let (mut body_graph, inputs) = GraphWithFuncs::new(
            loop_in_tys.len(),
            Some("should_continue".to_string()),
            loop_in_tys.len(),
        );
        // Replace block_preds: body of loop does not contain any branch from a BB outside it
        let mut body_preds = HashMap::new();
        let (exit_val, mut block_outputs) =
            self.build_nonloop(&mut body_graph, hugr, bb, inputs.clone(), &mut body_preds)?;
        assert!(exit_val.is_none()); // Exit block is not in the loop, so even if it was in `children`
        // it will have been moved out of `children` into `loop_.post_loop`
        let (mut input_port_names, output_names) = build_loop_repeat_val(
            hugr,
            loop_,
            bb,
            &mut body_graph,
            &block_outputs,
            body_preds,
            inputs,
        );
        input_port_names.insert(0, "graph".to_string());
        let graph_const_node = outer_graph.graph.add_node(
            graph_const(body_graph.graph)?,
            vec![],
            vec!["value".to_string()],
        );
        let eval_node = outer_graph.graph.add_node(
            NodeDefinition::Loop {},
            input_port_names.clone(),
            output_names.clone(),
        );
        wire_up(
            &mut outer_graph.graph,
            iter::once(&(graph_const_node, "value".to_string())).chain(&this_block_inputs),
            eval_node,
            &input_port_names,
        );
        match loop_.post_loop.as_ref() {
            Some(dtn) => dtn.build_graph(
                outer_graph,
                hugr,
                output_names.into_iter().map(|p| (eval_node, p)).collect(),
                block_preds,
            ),
            None => Ok((
                None,
                leaves(&self.exit_edges, hugr)
                    .into_iter()
                    .map(|lp| (lp.src, block_outputs.remove(&lp.src).unwrap()))
                    .collect(),
            )),
        }
    }

    fn build_nonloop(
        &self,
        graph: &mut GraphWithFuncs<N>,
        hugr: &impl HugrView<Node = N>,
        bb: &hugr::ops::DataflowBlock,
        inputs: Vec<(NodeIndex, String)>,
        block_preds: &mut HashMap<N, (NodeIndex, String)>,
    ) -> miette::Result<(
        Option<Vec<(NodeIndex, String)>>, // any values delivered to exit node (only if in this DomTreeNode)
        HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>, // values delivered to control-flow outports of various descendants
    )> {
        let mut block_outputs: HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>> =
            HashMap::new();

        // Compile body. (Easy - the complexity of this function is all about the branches!)
        let this_block_outs = convert_dfg(hugr, self.node, graph, inputs.clone())?;
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
            let (exit_block_outs, exit_edge_outs) =
                child.build_graph(graph, hugr, child_inputs, block_preds)?;
            if let Some(exit_block_outs) = exit_block_outs {
                let prev = exit_node_outs.replace(exit_block_outs);
                assert!(prev.is_none())
            }
            block_outputs.extend(exit_edge_outs);
        }

        Ok((exit_node_outs, block_outputs))
    }
}

// Builds, in `body_graph`, the predicate and repeat-inputs wired to the loop body's output;
// returns the (graph-const eval) input port names and the loop body's output port names.
fn build_loop_repeat_val<N: HugrNode>(
    hugr: &impl HugrView<Node = N>,
    loop_: &Loop<N>,
    bb: &hugr::ops::DataflowBlock,
    body_graph: &mut GraphWithFuncs<N>,
    block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
    body_preds: HashMap<N, (NodeIndex, String)>,
    inputs: Vec<(NodeIndex, String)>,
) -> (Vec<String>, Vec<String>) {
    let loop_in_tys = &bb.inputs;
    let [loop_exit_edge] = loop_.exit_path.leaves(hugr).try_into().unwrap();
    let loop_exit_tys = hugr
        .get_optype(loop_exit_edge.src.0)
        .as_dataflow_block()
        .unwrap()
        .successor_input(loop_exit_edge.src.1.index())
        .unwrap();
    assert_eq!(&loop_exit_tys, loop_in_tys); // TODO: allow exitting with only a subset? But, how to identify?
    let does_loop_repeat = loop_.repeat_path.build_predicate(
        &mut body_graph.graph,
        &body_preds,
        &mut None, // search for existing in WorkflowGraph??
        &mut None, // probably not worth it, graph only contains this loop body
    );
    let mut loop_exit_path = loop_.exit_path.clone();
    loop_exit_path.union(&loop_.repeat_path);
    let rep_val = loop_exit_path.build_inputs(&mut body_graph.graph, &block_outputs, &body_preds);
    assert_eq!(rep_val.len(), loop_in_tys.len());
    let output_node = body_graph.graph.output_node;
    let output_names: Vec<String> = body_graph
        .graph
        .input_names(body_graph.graph.output_node)
        .unwrap()
        .cloned()
        .collect();
    wire_up(
        &mut body_graph.graph,
        iter::once(&does_loop_repeat).chain(&rep_val),
        output_node,
        &output_names,
    );
    let input_port_names = inputs.into_iter().map(|(node, port)|{
        assert!(
            matches!(body_graph.graph.node_definition(node), Some(NodeDefinition::Input { name }) if *name == port)
        );
        port
    }).collect();
    (input_port_names, output_names)
}

fn find_single_loop_exit<H: HugrView>(
    hugr: &H,
    loop_header: H::Node,
    backedges: &GatingPath<H::Node>,
) -> (H::Node, OutgoingPort) {
    let mut blocks = HashSet::new();
    let mut queue = VecDeque::from_iter(backedges.leaves(hugr).into_iter().map(|lp| lp.src.0));
    while let Some(n) = queue.pop_front() {
        if !blocks.insert(n) || n == loop_header {
            continue;
        }
        queue.extend(hugr.input_neighbours(n));
    }
    let loop_exits = blocks
        .iter()
        .flat_map(|&n| hugr.node_outputs(n).map(move |p| (n, p)))
        .filter(|(n, p)| !blocks.contains(&hugr.single_linked_input(*n, *p).unwrap().0))
        .collect::<Vec<_>>();
    let Ok(res) = loop_exits.into_iter().exactly_one() else {
        // Need to build a single exit that collects all these together, with a sum type or similar
        // that identifies which exit was taken and where to go next
        todo!("Multi-exit loops")
    };
    res
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
        let mut exit_edges: Option<GatingPath<H::Node>> = None;
        let mut loop_backedges: Option<GatingPath<H::Node>> = None;

        let mut child_paths = HashMap::<H::Node, GatingPath<H::Node>>::new();
        // Process edges from this node (perhaps a loop header)
        let path = hugr
            .node_outputs(n.into())
            .exactly_one()
            .ok()
            .map(|p| GatingPath::Always(n, p));
        let outports = hugr.node_outputs(n.into()).collect::<Vec<_>>();
        for outport in &outports {
            let path = path
                .clone()
                .unwrap_or(GatingPath::branch(n, *outport, outports.len()));
            // Control Flow outports should have exactly one outgoing edge
            let (tgt, _) = hugr
                .linked_inputs(n.into(), *outport)
                .exactly_one()
                .ok()
                .unwrap();
            if children_by_bb.contains_key(&tgt) {
                child_paths
                    .entry(tgt)
                    .and_modify(|p| p.union(&path))
                    .or_insert_with(|| path.clone());
            } else if tgt == n {
                union_opt(&mut loop_backedges, &path);
            } else {
                union_opt(&mut exit_edges, &path)
            }
        }
        // We want to process children in reverse topsort order: any child C1 with an exit edge to C2, must be processed *before* C2.
        let mut ordered_children = Vec::new();
        fn rev_sort<N: HugrNode>(
            hugr: &impl HugrView<Node = N>,
            ordered: &mut Vec<N>,
            child: N,
            remaining_children: &mut HashMap<N, &DomTreeNode<N>>,
        ) {
            let Some(dtn) = remaining_children.remove(&child) else {
                return;
            };
            // Targets of exit edges pushed onto <ordered> first
            for lp in leaves(&dtn.exit_edges, hugr) {
                rev_sort(hugr, ordered, lp.tgt, remaining_children);
            }
            ordered.push(child);
        }
        let mut remaining_children = children_by_bb.iter().map(|(n, c)| (*n, c)).collect();
        for child in children_by_bb.values() {
            rev_sort(
                &hugr,
                &mut ordered_children,
                child.node,
                &mut remaining_children,
            );
        }
        // This means targets of exit edges will be processed *after* all the sources of said exit edges:
        ordered_children.reverse();

        // Now process children in the determined order
        let mut children_by_bb = children_by_bb;
        let children = ordered_children
            .into_iter()
            .map(|child| {
                let path_to_child = child_paths.remove(&child).unwrap();
                let child_dtn = children_by_bb.remove(&child).unwrap();
                let child_exit_leaves = leaves(&child_dtn.exit_edges, hugr);
                for lp in child_exit_leaves {
                    assert!(
                        // if dst has no dominator, dst is the entry node
                        doms.immediate_dominator(node_map.to_portgraph(lp.tgt))
                            .is_none_or(|tgt_dom|
                        //otherwise, tgt_dom must be ni or some dominator thereof
                        // (i.e. tgt is a sibling of an nonstrict-ancestor of ni).
                    doms.dominators(ni).unwrap().contains(&tgt_dom))
                    );
                    let path_to_exit = path_to_child.concat(&lp.clone().into());
                    if children_by_bb.contains_key(&lp.tgt) {
                        child_paths
                            .entry(lp.tgt)
                            .and_modify(|p| p.union(&path_to_exit))
                            .or_insert_with(|| path_to_exit.clone());
                    } else if lp.tgt == n {
                        union_opt(&mut loop_backedges, &path_to_exit);
                    } else {
                        union_opt(&mut exit_edges, &path_to_exit);
                    }
                }
                (path_to_child, child_dtn)
            })
            .collect();

        let mut d = DomTreeNode {
            node: n,
            children,
            exit_edges,
            loop_: None,
        };
        if let Some(path_back_to_header) = loop_backedges {
            // This node is a loop header. It dominates the entire loop body, so any edge
            // exitting the DomTree exits the loop.
            // The header but may also dominate some bits not in the loop (i.e. from which
            // control cannot flow back to the header - at least not without exitting the
            // DomTree and going round an *outer* loop to this one.)
            let (loop_exit_block, outport) = find_single_loop_exit(hugr, n, &path_back_to_header);
            let (post_loop, _) = hugr.single_linked_input(loop_exit_block, outport).unwrap();
            let mut post_loop_doms = Vec::new();
            let mut dom = post_loop;
            let (loop_exit_path, post_loop_dtn) = loop {
                post_loop_doms.push(dom);
                match doms.immediate_dominator(node_map.to_portgraph(dom)) {
                    Some(ni) => {
                        dom = node_map.from_portgraph(ni);
                        if dom == n {
                            // Unique loop_ block is dominated by header.
                            // Any edge exiting the DomTree exits the loop; so must come from that post-loop block.
                            for exit_edge in leaves(&d.exit_edges, hugr) {
                                assert!(
                                    doms.dominators(node_map.to_portgraph(exit_edge.src.0))
                                        .unwrap()
                                        .contains(&node_map.to_portgraph(post_loop))
                                );
                            }
                            post_loop_doms.reverse();
                            let (ep, subtree) = d.disconnect(&post_loop_doms, true);
                            break (ep, Some(subtree));
                        }
                    }
                    None => {
                        // loop_ edge leaves the subtree
                        let [tree_exit_edge] = leaves(&d.exit_edges, hugr).try_into().unwrap();
                        assert_eq!(tree_exit_edge.src, (loop_exit_block, outport));
                        d.exit_edges = None;
                        break (tree_exit_edge.into(), None);
                    }
                }
            };
            d.loop_ = Some(Loop {
                repeat_path: path_back_to_header,
                exit_path: loop_exit_path,
                post_loop: post_loop_dtn.map(Box::new),
            });
        }
        d
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
    let (outs, _) = tr.build_graph(graph, hugr, inputs, &mut HashMap::new())?;
    Ok(outs.unwrap())
}

/// A collection (0 or more) paths from some given CFG node, within that node's dominator tree.
/// `None` (rather than a `GatingPath`) represents the empty collection (no paths).
#[derive(Clone, Debug)]
enum GatingPath<N> {
    // Source of an edge leaving the dom tree
    Always(N, OutgoingPort),
    // One element (None if no path via that port) for each outgoing port of the branch node
    Branch(N, Vec<Option<GatingPath<N>>>),
}

/// A single path from some given CFG node, stopping at the boundary of its dominator tree.
#[derive(Clone, Debug, Default)]
struct LeafPath<N> {
    /// Previous branches passed through on the way to [Self::src]. The `usize` caches the number of outgoing ports.
    branches: Vec<(N, OutgoingPort, usize)>,
    /// The source node (last node dominated by the start), and the outgoing port
    /// whose edge leaves the dominator tree
    src: (N, OutgoingPort),
    /// Target of that edge (outside the dominator tree)
    tgt: N,
}

impl<N> From<LeafPath<N>> for GatingPath<N> {
    fn from(leaf: LeafPath<N>) -> Self {
        let mut path = GatingPath::Always(leaf.src.0, leaf.src.1);
        for (node, port, len) in leaf.branches.into_iter().rev() {
            let mut branches = Vec::from_iter(iter::repeat_with(|| None).take(len));
            branches[port.index()] = Some(path);
            path = GatingPath::Branch(node, branches);
        }
        path
    }
}

/// Merges `other` into the accumulated path `acc`, which may not have any path yet.
fn union_opt<N: HugrNode>(acc: &mut Option<GatingPath<N>>, other: &GatingPath<N>) {
    match acc {
        Some(existing) => existing.union(other),
        None => *acc = Some(other.clone()),
    }
}

/// Removes `to_remove` from `from`, panicking if `from` is `None` or does not contain `to_remove`.
fn remove_opt<N: HugrNode>(from: &mut Option<GatingPath<N>>, to_remove: &GatingPath<N>) {
    *from = from
        .take()
        .expect("Cannot remove from None")
        .remove(to_remove);
}

impl<N: HugrNode> GatingPath<N> {
    fn branch(node: N, port: OutgoingPort, num_ports: usize) -> Self {
        let mut branches = vec![None; num_ports];
        branches[port.index()] = Some(GatingPath::Always(node, port));
        GatingPath::Branch(node, branches)
    }

    fn build_predicate(
        &self,
        graph: &mut WorkflowGraph,
        block_preds: &HashMap<N, (NodeIndex, String)>,
        const_true: &mut Option<(NodeIndex, String)>,
        const_false: &mut Option<(NodeIndex, String)>,
    ) -> (NodeIndex, String) {
        match self {
            GatingPath::Always(_, _) => const_true
                .get_or_insert_with(|| {
                    (
                        graph.add_node(
                            NodeDefinition::Const { value: true.into() },
                            [],
                            ["value".to_string()],
                        ),
                        "value".to_string(),
                    )
                })
                .clone(),
            GatingPath::Branch(br, opts) => {
                let [fal, tru] = &opts[..] else {
                    panic!("Branches should have two options, guppy only branches on bools")
                };
                let [fal, tru] = [fal, tru].map(|path| match path {
                    None => const_false
                        .get_or_insert_with(|| {
                            (
                                graph.add_node(
                                    NodeDefinition::Const {
                                        value: false.into(),
                                    },
                                    [],
                                    ["value".to_string()],
                                ),
                                "value".to_string(),
                            )
                        })
                        .clone(),
                    Some(gp) => gp.build_predicate(graph, block_preds, const_true, const_false),
                });
                let pred = block_preds.get(br).unwrap();
                let node = graph.add_node(
                    NodeDefinition::IfElse {},
                    [
                        "pred".to_string(),
                        "if_true".to_string(),
                        "if_false".to_string(),
                    ],
                    ["value".to_string()],
                );
                wire_up(
                    graph,
                    [pred, &fal, &tru],
                    node,
                    ["pred", "if_false", "if_true"],
                );
                (node, "value".into())
            }
        }
    }

    fn build_inputs(
        &self,
        graph: &mut WorkflowGraph,
        block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
        block_preds: &HashMap<N, (NodeIndex, String)>,
    ) -> Vec<(NodeIndex, String)> {
        match self {
            GatingPath::Always(n, p) => block_outputs.get(&(*n, *p)).unwrap().clone(),
            GatingPath::Branch(br, opts) => {
                assert_eq!(opts.len(), 2); // guppy only produces bools
                if let Ok(path) = opts.iter().flatten().exactly_one() {
                    return path.build_inputs(graph, block_outputs, block_preds);
                }
                let (Some(fal), Some(tr)) = (&opts[0], &opts[1]) else {
                    panic!("Some branch must be present")
                };
                let fal = fal.build_inputs(graph, block_outputs, block_preds);
                let tr = tr.build_inputs(graph, block_outputs, block_preds);
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
                            wire_up(
                                graph,
                                [pred, f, t],
                                node,
                                vec!["pred", "if_false", "if_true"],
                            );
                            (node, "value".into())
                        }
                    })
                    .collect()
            }
        }
    }

    fn concat(&self, other: &GatingPath<N>) -> Self {
        fn has_none<N>(gp: &Option<GatingPath<N>>) -> bool {
            match gp {
                None => true,
                Some(GatingPath::Always(_, _)) => false,
                Some(GatingPath::Branch(_, opts)) => opts.iter().any(has_none),
            }
        }
        match self {
            GatingPath::Always(_, _) => other.clone(),
            GatingPath::Branch(node, opts) => {
                if opts.iter().any(has_none) {
                    GatingPath::Branch(
                        *node,
                        opts.iter()
                            .map(|v| v.as_ref().map(|v| v.concat(other)))
                            .collect(),
                    )
                } else {
                    other.clone()
                }
            }
        }
    }

    fn leaves(&self, hugr: &impl HugrView<Node = N>) -> Vec<LeafPath<N>> {
        fn traverse<H: HugrView>(
            hugr: &H,
            gp: &GatingPath<H::Node>,
            path_to_here: &mut Vec<(H::Node, OutgoingPort, usize)>,
        ) -> Vec<LeafPath<H::Node>> {
            match gp {
                GatingPath::Always(node, port) => {
                    let (tgt, _) = hugr.single_linked_input(*node, *port).unwrap();
                    vec![LeafPath {
                        branches: path_to_here.clone(),
                        src: (*node, *port),
                        tgt,
                    }]
                }
                GatingPath::Branch(br, opts) => opts
                    .iter()
                    .enumerate()
                    .flat_map(|(i, gp)| {
                        path_to_here.push((*br, i.into(), opts.len()));
                        let leaves = gp
                            .as_ref()
                            .map(|gp| traverse(hugr, gp, path_to_here))
                            .unwrap_or_default();
                        path_to_here.pop();
                        leaves
                    })
                    .collect(),
            }
        }
        traverse(hugr, self, &mut Vec::new())
    }

    fn union(&mut self, other: &GatingPath<N>) {
        match self {
            GatingPath::Always(_, _) => {
                panic!("Union of Always with {other:?}");
            }
            GatingPath::Branch(n, opts) => {
                if let GatingPath::Branch(n2, opts2) = other
                    && n == n2
                {
                    for (this, other) in opts.iter_mut().zip_eq(opts2.iter()) {
                        let Some(other) = other else { continue };
                        match this {
                            None => *this = Some(other.clone()),
                            Some(this) => this.union(other),
                        }
                    }
                } else {
                    panic!("Cannot union Branch({n:?}) with {other:?}");
                }
            }
        }
    }

    fn remove(mut self, other: &GatingPath<N>) -> Option<GatingPath<N>> {
        match &mut self {
            GatingPath::Always(n, p) => {
                if matches!(other, GatingPath::Always(n2, p2) if n==n2 && p==p2) {
                    return None;
                }
            }
            GatingPath::Branch(n, opts) => {
                if let GatingPath::Branch(n2, opts2) = other
                    && n == n2
                {
                    for (this, other) in opts.iter_mut().zip_eq(opts2.iter()) {
                        let Some(other) = other else { continue };
                        remove_opt(this, other);
                    }
                    return Some(self);
                }
            }
        }
        panic!("Can't remove {other:?} from {self:?}");
    }
}

fn leaves<H: HugrView>(gp: &Option<GatingPath<H::Node>>, hugr: &H) -> Vec<LeafPath<H::Node>> {
    match gp {
        Some(gp) => gp.leaves(hugr),
        None => Vec::new(),
    }
}
