use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter;

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort, PortIndex as _};
use portgraph::NodeIndex;
use tket::passes::cfg::dom_tree::{DomTreeNode, DomTreeWithBackedges, Path};
use tket::passes::cfg::gating_path::GatingPath;

use super::{GraphWithFuncs, convert_dfg, graph_const, wire_up};
use crate::graph::{NodeDefinition, WorkflowGraph};

// TODO: make exit_path into LeafPath, and put inside the Option (if None,
// the exit_path is the unique exit edge of the parent DomTreeNode).
struct Loop<N: HugrNode> {
    repeat_path: GatingPath<N>,
    exit_path: GatingPath<N>,
    post_loop: Option<Box<DomTree<N>>>,
}

type DomTree<N> = DomTreeNode<N, GatingPath<N>, Option<Loop<N>>>;

fn disconnect<N: HugrNode>(
    dtn: &mut DomTreeWithBackedges<N, GatingPath<N>>,
    doms: &[N],
    outer: bool,
) -> (GatingPath<N>, DomTreeWithBackedges<N, GatingPath<N>>) {
    for (child_idx, (child_path, child)) in dtn.children.iter_mut().enumerate() {
        if child.node == doms[0] {
            let (ep, dtn2) = if doms.len() == 1 {
                dtn.children.remove(child_idx)
                // Note ep *is* child_path
            } else {
                let (ep, dtn) = disconnect(child, &doms[1..], false);
                (child_path.concat(&ep), dtn)
            };
            if !outer {
                // Remove paths to leaves of the removed subtree, add a path to the root thereof
                if let Some(ees) = dtn2.exit_edges.as_ref() {
                    let to_remove = ep.clone().concat(ees);
                    remove_opt(&mut dtn.exit_edges, &to_remove);
                }
                GatingPath::union_opt(&mut dtn.exit_edges, &ep);
            }
            return (ep, dtn2);
        }
    }
    if !outer && dtn.loop_.is_some()
    /*&& let Some(post_loop) = loop_.post_loop.as_mut()
    && post_loop.node == doms[0]*/
    {
        panic!("Nested loop!"); // Just poison, don't think we're hitting this yet

        /*if doms.len() == 1 {
            return (loop_.exit_path.clone(), *loop_.post_loop.take().unwrap());
        }
        let (ep, dtn) = disconnect(post_loop, &doms[1..], false);
        return (loop_.exit_path.concat(&ep), dtn);*/
    }
    panic!("Node not found in children");
}

fn build_graph<N: HugrNode>(
    dtn: &DomTree<N>,
    graph: &mut GraphWithFuncs<N>,
    hugr: &impl HugrView<Node = N>,
    this_block_inputs: Vec<(NodeIndex, String)>,
    block_preds: &mut HashMap<N, (NodeIndex, String)>,
) -> miette::Result<(
    Option<Vec<(NodeIndex, String)>>, // any values delivered to exit node (only if in this DomTreeNode)
    HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>, // values delivered to exit edges of this DomTreeNode
)> {
    let Some(bb) = hugr.get_optype(dtn.node).as_dataflow_block() else {
        assert!(hugr.get_optype(dtn.node).is_exit_block());
        assert!(dtn.exit_edges.is_none());
        assert!(dtn.children.is_empty());
        return Ok((Some(this_block_inputs), HashMap::new()));
    };
    let Some(loop_) = dtn.loop_.as_ref() else {
        let (exit_node_vals, mut block_outputs) =
            build_nonloop(dtn, graph, hugr, bb, this_block_inputs, block_preds)?;
        return Ok((
            exit_node_vals,
            leaves(dtn.exit_edges.as_ref())
                .into_iter()
                .map(|(src, _)| (src, block_outputs.remove(&src).unwrap()))
                .collect(),
        ));
    };
    let outer_graph = graph; // Rename to disambiguate.

    let loop_in_tys = &hugr
        .get_optype(dtn.node)
        .as_dataflow_block()
        .unwrap()
        .inputs;
    let loop_val_names = (0..loop_in_tys.len())
        .map(|i| format!("var{i}"))
        .collect::<Vec<_>>();

    let body_outputs = iter::once("should_continue".to_string())
        .chain(loop_val_names.iter().cloned())
        .collect::<Vec<_>>();
    let (mut body_graph, inputs) =
        GraphWithFuncs::new_with_names(loop_val_names.clone(), body_outputs.clone());
    // Replace block_preds: body of loop does not contain any branch from a BB outside it
    let mut body_preds = HashMap::new();
    let (exit_val, mut block_outputs) = build_nonloop(
        dtn,
        &mut body_graph,
        hugr,
        bb,
        inputs.clone(),
        &mut body_preds,
    )?;
    assert!(exit_val.is_none()); // Exit block is not in the loop, so even if it was in `children`
    // it will have been moved out of `children` into `loop_.post_loop`
    build_loop_repeat_val(
        hugr,
        loop_,
        bb,
        &mut body_graph,
        &body_outputs,
        &block_outputs,
        body_preds,
        inputs,
    );
    let input_port_names = iter::once("graph".to_string())
        .chain(loop_val_names.iter().cloned())
        .collect::<Vec<_>>();
    let graph_const_node = outer_graph.graph.add_node(
        graph_const(body_graph.graph)?,
        vec![],
        vec!["value".to_string()],
    );
    let eval_node = outer_graph.graph.add_node(
        NodeDefinition::Loop {},
        input_port_names.clone(),
        loop_val_names.clone(),
    );
    wire_up(
        &mut outer_graph.graph,
        iter::once(&(graph_const_node, "value".to_string())).chain(&this_block_inputs),
        eval_node,
        &input_port_names,
    );
    match loop_.post_loop.as_ref() {
        Some(dtn) => build_graph(
            &**dtn,
            outer_graph,
            hugr,
            loop_val_names.into_iter().map(|p| (eval_node, p)).collect(),
            block_preds,
        ),
        None => {
            panic!(
                "TODO Hmmmm, we need to put the Loop node outputs into block_outputs somewhere."
            );
            Ok((
                None,
                leaves(dtn.exit_edges.as_ref())
                    .into_iter()
                    .map(|(src, _)| (src, block_outputs.remove(&src).unwrap()))
                    .collect(),
            ))
        }
    }
}

fn build_nonloop<N: HugrNode>(
    dtn: &DomTree<N>,
    graph: &mut GraphWithFuncs<N>,
    hugr: &impl HugrView<Node = N>,
    bb: &hugr::ops::DataflowBlock,
    inputs: Vec<(NodeIndex, String)>,
    block_preds: &mut HashMap<N, (NodeIndex, String)>,
) -> miette::Result<(
    Option<Vec<(NodeIndex, String)>>, // any values delivered to exit node (only if in this DomTreeNode)
    HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>, // values delivered to control-flow outports of various descendants
)> {
    let mut block_outputs: HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>> = HashMap::new();

    // Compile body. (Easy - the complexity of this function is all about the branches!)
    let this_block_outs = convert_dfg(hugr, dtn.node, graph, inputs.clone())?;
    // Edges from "root"
    assert_eq!(hugr.node_outputs(dtn.node).count(), bb.sum_rows.len());
    // Guppy generates only unit sum branch predicates
    assert!(bb.sum_rows.iter().all(|row| row.is_empty()));
    assert!([1, 2].contains(&bb.sum_rows.len())); // For now we support only two-way branches
    if bb.sum_rows.len() == 2 {
        block_preds.insert(dtn.node, this_block_outs[0].clone());
    }
    // Do not include predicate (the first output) in the block outputs - any predicate
    // necessary will be built by the relevant GatingPath reading from block_preds
    for p in hugr.node_outputs(dtn.node) {
        block_outputs.insert((dtn.node, p), this_block_outs[1..].to_vec());
    }

    let mut exit_node_outs = None;

    for (child_path, child) in &dtn.children {
        let child_inputs = build_inputs(child_path, &mut graph.graph, &block_outputs, &block_preds);
        let (exit_block_outs, exit_edge_outs) =
            build_graph(child, graph, hugr, child_inputs, block_preds)?;
        if let Some(exit_block_outs) = exit_block_outs {
            let prev = exit_node_outs.replace(exit_block_outs);
            assert!(prev.is_none())
        }
        block_outputs.extend(exit_edge_outs);
    }

    Ok((exit_node_outs, block_outputs))
}

// Builds, in `body_graph`, the predicate and repeat-inputs wired to the loop body's output
fn build_loop_repeat_val<N: HugrNode>(
    hugr: &impl HugrView<Node = N>,
    loop_: &Loop<N>,
    bb: &hugr::ops::DataflowBlock,
    body_graph: &mut GraphWithFuncs<N>,
    body_outputs: &Vec<String>,
    block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
    body_preds: HashMap<N, (NodeIndex, String)>,
    inputs: Vec<(NodeIndex, String)>,
) {
    let loop_in_tys = &bb.inputs;
    let [((loop_exit_src, loop_exit_port), _)] = loop_.exit_path.leaves().collect_array().unwrap();
    let loop_exit_tys = hugr
        .get_optype(loop_exit_src)
        .as_dataflow_block()
        .unwrap()
        .successor_input(loop_exit_port.index())
        .unwrap();
    assert_eq!(&loop_exit_tys, loop_in_tys); // TODO: allow exitting with only a subset? But, how to identify?
    let does_loop_repeat = build_predicate(
        &loop_.repeat_path,
        &mut body_graph.graph,
        &body_preds,
        &mut None, // search for existing in WorkflowGraph??
        &mut None, // probably not worth it, graph only contains this loop body
    );
    let mut loop_exit_path = loop_.exit_path.clone();
    loop_exit_path.union(&loop_.repeat_path);
    let rep_val = build_inputs(
        &loop_exit_path,
        &mut body_graph.graph,
        &block_outputs,
        &body_preds,
    );
    assert_eq!(rep_val.len(), loop_in_tys.len());
    let output_node = body_graph.graph.output_node;

    wire_up(
        &mut body_graph.graph,
        iter::once(&does_loop_repeat).chain(&rep_val),
        output_node,
        body_outputs,
    );
}

fn find_single_loop_exit<H: HugrView>(
    hugr: &H,
    loop_header: H::Node,
    backedges: &GatingPath<H::Node>,
) -> (H::Node, OutgoingPort) {
    let mut blocks = HashSet::new();
    let mut queue = VecDeque::from_iter(
        backedges
            .leaves()
            .into_iter()
            .map(|((node, _port), _)| node),
    );
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

fn dominator_map<N: HugrNode, P: Path<N>, L>(tr: &DomTreeNode<N, P, L>) -> HashMap<N, N> {
    let mut map = HashMap::new();
    for (_, ch) in &tr.children {
        map.extend(dominator_map(ch));
        map.insert(ch.node, tr.node);
    }
    map
}

fn convert_loops<N: HugrNode>(
    mut tr: DomTreeWithBackedges<N, GatingPath<N>>,
    hugr: &impl HugrView<Node = N>,
) -> DomTree<N> {
    let doms = dominator_map(&tr);
    let conv_children = |children: Vec<(GatingPath<N>, DomTreeWithBackedges<N, GatingPath<N>>)>| {
        children
            .into_iter()
            .map(|(p, c)| (p, convert_loops(c, hugr)))
            .collect::<Vec<_>>()
    };
    let Some(path_back_to_header) = tr.loop_.take() else {
        return DomTree {
            node: tr.node,
            exit_edges: tr.exit_edges,
            children: conv_children(tr.children),
            loop_: None,
        };
    };

    // This node is a loop header. It dominates the entire loop body, so any edge
    // exitting the DomTree exits the loop.
    // The header but may also dominate some bits not in the loop (i.e. from which
    // control cannot flow back to the header - at least not without exitting the
    // DomTree and going round an *outer* loop to this one.)
    let (loop_exit_block, outport) = find_single_loop_exit(hugr, tr.node, &path_back_to_header);
    let (post_loop, _) = hugr.single_linked_input(loop_exit_block, outport).unwrap();

    let mut post_loop_doms = Vec::new();
    let mut n = post_loop;
    let (loop_exit_path, post_loop_dtn) = loop {
        post_loop_doms.push(n);
        match doms.get(&n) {
            Some(dom) => {
                if *dom == tr.node {
                    // Unique loop_ block is dominated by header.
                    // Any edge exiting the DomTree exits the loop; so must come from that post-loop block.
                    /*for exit_edge in leaves(&d.exit_edges, hugr) {
                        assert!(
                            doms.dominators(node_map.to_portgraph(exit_edge.src.0))
                                .unwrap()
                                .contains(&node_map.to_portgraph(post_loop))
                        );
                    }*/
                    post_loop_doms.reverse();
                    let (ep, subtree) = disconnect(&mut tr, &post_loop_doms, true);
                    break (ep, Some(convert_loops(subtree, hugr)));
                }
                n = *dom;
            }
            None => {
                // loop_ edge leaves the subtree
                let [(tree_exit_edge, exit_path)] =
                    leaves(tr.exit_edges.as_ref()).collect_array().unwrap();
                assert_eq!(tree_exit_edge, (loop_exit_block, outport));
                tr.exit_edges = None;
                break (exit_path, None);
            }
        }
    };
    DomTree {
        node: tr.node,
        children: conv_children(tr.children),
        exit_edges: tr.exit_edges,
        loop_: Some(Loop {
            repeat_path: path_back_to_header,
            exit_path: loop_exit_path,
            post_loop: post_loop_dtn.map(Box::new),
        }),
    }
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
    let tr = DomTreeWithBackedges::<H::Node, GatingPath<H::Node>>::new_for_cfg(hugr, node);

    let tr = convert_loops(tr, hugr);
    let (outs, _) = build_graph(&tr, graph, hugr, inputs, &mut HashMap::new())?;
    Ok(outs.unwrap())
}

/// Removes `to_remove` from `from`, panicking if `from` is `None` or does not contain `to_remove`.
fn remove_opt<N: HugrNode>(from: &mut Option<GatingPath<N>>, to_remove: &GatingPath<N>) {
    *from = remove(from.take().expect("Cannot remove from None"), to_remove);
}

fn build_predicate<N: HugrNode>(
    gp: &GatingPath<N>,
    graph: &mut WorkflowGraph,
    block_preds: &HashMap<N, (NodeIndex, String)>,
    const_true: &mut Option<(NodeIndex, String)>,
    const_false: &mut Option<(NodeIndex, String)>,
) -> (NodeIndex, String) {
    match gp {
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
                Some(gp) => build_predicate(gp, graph, block_preds, const_true, const_false),
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

fn build_inputs<N: HugrNode>(
    gp: &GatingPath<N>,
    graph: &mut WorkflowGraph,
    block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
    block_preds: &HashMap<N, (NodeIndex, String)>,
) -> Vec<(NodeIndex, String)> {
    match gp {
        GatingPath::Always(n, p) => block_outputs.get(&(*n, *p)).unwrap().clone(),
        GatingPath::Branch(br, opts) => {
            assert_eq!(opts.len(), 2); // guppy only produces bools
            if let Ok(path) = opts.iter().flatten().exactly_one() {
                return build_inputs(path, graph, block_outputs, block_preds);
            }
            let (Some(fal), Some(tr)) = (&opts[0], &opts[1]) else {
                panic!("Some branch must be present")
            };
            let fal = build_inputs(fal, graph, block_outputs, block_preds);
            let tr = build_inputs(tr, graph, block_outputs, block_preds);
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

fn remove<N: HugrNode>(mut gp: GatingPath<N>, other: &GatingPath<N>) -> Option<GatingPath<N>> {
    match &mut gp {
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
                return Some(gp);
            }
        }
    }
    panic!("Can't remove {other:?} from {gp:?}");
}

fn leaves<N: HugrNode>(
    gp: Option<&GatingPath<N>>,
) -> impl Iterator<Item = ((N, OutgoingPort), GatingPath<N>)> {
    gp.into_iter().flat_map(|gp| gp.leaves())
}
