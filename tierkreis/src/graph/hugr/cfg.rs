use itertools::Itertools;
use std::collections::HashMap;

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort};
use portgraph::NodeIndex;
use tket::passes::cfg::dom_tree::{DomTreeWithBackedges, Path};
use tket::passes::cfg::gating_path::GatingPath;

use super::{GraphWithFuncs, convert_dfg, wire_up};
use crate::graph::WorkflowGraph;

type DomTree<N> = DomTreeWithBackedges<N, GatingPath<N>>;

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
    if GatingPath::opt_as_ref(&dtn.loop_).is_some() {
        return Err(miette::miette!(
            "CFG had loop; use TKET2 `nest_loop` or `RemoveCFGCyclesPass` first"
        ));
    }

    // values delivered to control-flow outports of various descendants
    let mut block_outputs: HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>> = HashMap::new();

    // Compile body. (Easy - the complexity of this function is all about the branches!)
    let this_block_outs = convert_dfg(hugr, dtn.node, graph, this_block_inputs.clone())?;
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

    // Any values delivered to the CFG's ExitBlock (iff in this DomTreeNode)
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

    Ok((
        exit_node_outs,
        leaves(dtn.exit_edges.as_ref())
            .into_iter()
            .map(|(src, _)| (src, block_outputs.remove(&src).unwrap()))
            .collect(),
    ))
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
    let tr = DomTreeWithBackedges::new_for_cfg(hugr, node);

    let (outs, _) = build_graph(&tr, graph, hugr, inputs, &mut HashMap::new())?;
    Ok(outs.unwrap())
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
                        let node = crate::builder::if_else(graph);
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

fn leaves<N: HugrNode>(
    gp: Option<&GatingPath<N>>,
) -> impl Iterator<Item = ((N, OutgoingPort), GatingPath<N>)> {
    gp.into_iter().flat_map(|gp| gp.leaves())
}
