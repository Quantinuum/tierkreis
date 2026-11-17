use itertools::Itertools;
use std::collections::HashMap;

use hugr::{HugrView, OutgoingPort, core::HugrNode, types::TypeRow};
use portgraph::NodeIndex;
use tket::passes::cfg::dom_tree::{DomTreeWithBackedges, Path};

use super::{GraphWithFuncs, convert_dfg, gating_path::GatingPath, unpack_tuple, wire_up};
use crate::builder::{constant, if_else, task};
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

    // Compile body. (Easy - the complexity of this function is all about the branches!)
    let this_block_outs = convert_dfg(hugr, dtn.node, graph, this_block_inputs.clone())?;

    // Edges from "root"
    assert_eq!(hugr.node_outputs(dtn.node).count(), bb.sum_rows.len());

    let (predicate, outputs) = build_block_outputs(
        graph,
        hugr.node_outputs(dtn.node),
        this_block_outs,
        &bb.sum_rows,
    );
    block_preds.extend(predicate.map(|p| (dtn.node, p)));

    // values delivered to control-flow outports of various descendants
    let mut block_outputs = outputs
        .into_iter()
        .map(|(p, outs)| ((dtn.node, p), outs))
        .collect::<HashMap<_, _>>();

    // Any values delivered to the CFG's ExitBlock (iff in this DomTreeNode)
    let mut exit_node_outs = None;

    for (child_path, child) in &dtn.children {
        let child_inputs = build_inputs(
            child_path,
            &mut graph.graph,
            hugr,
            &block_outputs,
            &block_preds,
        );
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

/// Generates per-port outgoing values and optionally predicate for a block,
/// given the values provided to its Output node.
///
/// (Thus, handles the unpacking of the BB predicate performed at the end of a [DataflowBlock]).
///
/// Returns the predicate to insert into `block_preds` (if any), and the values
/// delivered to each control-flow outport of this block.
fn build_block_outputs<N: HugrNode>(
    graph: &mut GraphWithFuncs<N>,
    node_outports: impl Iterator<Item = OutgoingPort>,
    mut block_outputs: Vec<(NodeIndex, String)>,
    sum_rows: &[TypeRow],
) -> (
    Option<(NodeIndex, String)>,
    HashMap<OutgoingPort, Vec<(NodeIndex, String)>>,
) {
    let (mut predicate, this_block_outs) = (block_outputs.remove(0), block_outputs);

    // Generate outputs of BB for each successor (each gets different unpacking of Sum).
    if sum_rows.len() == 1 {
        let mut outs = unpack_tuple(&mut graph.graph, predicate, sum_rows[0].len());
        outs.extend(this_block_outs);
        return (None, HashMap::from([(OutgoingPort::from(0), outs)]));
    }

    // Handle arity > 2 by returning the tag (integer) for any non-bool predicate.
    let mut unpacked = Vec::new();
    if !predicate_is_bool(sum_rows) {
        let [tag, mut rest] = unpack_tuple(&mut graph.graph, predicate, 2)
            .try_into()
            .unwrap();
        predicate = tag;

        // Unpack the tuple only once for all values of `tag`. If some variants are shorter,
        // corresponding unpacks will be unused - fine under lazy evaluation.
        let max_elems = sum_rows.iter().map(|row| row.len()).max().unwrap_or(0);
        // Build vec where element [0] is unpacking of pair;
        // unpacking of triple is [0].0 followed by [1];
        // unpacking of quadruple is [0].0 followed by [1].0 followed by [2]; etc.
        let do_not_use = (
            NodeIndex::new(NodeIndex::<u32>::max_index()),
            "DO NOT USE".to_string(),
        );
        unpacked.push([do_not_use, rest.clone()]);

        for _ in 0..max_elems {
            let elem_rest: [_; 2] = unpack_tuple(&mut graph.graph, rest, 2).try_into().unwrap();
            [_, rest] = elem_rest.clone();
            unpacked.push(elem_rest);
        }
    }

    let block_outputs = node_outports
        .zip_eq(sum_rows)
        .map(|(p, pred_row)| {
            let outs = (1..pred_row.len())
                .map(|i| &unpacked[i])
                .map(|[e, _]| e.clone())
                .chain((!pred_row.is_empty()).then(|| unpacked[pred_row.len() - 1][1].clone()))
                .chain(this_block_outs.iter().cloned())
                .collect();
            (p, outs)
        })
        .collect();

    (Some(predicate), block_outputs)
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

fn predicate_is_bool(sum_rows: &[TypeRow]) -> bool {
    sum_rows == [TypeRow::new(), TypeRow::new()]
}

fn build_inputs<N: HugrNode>(
    gp: &GatingPath<N>,
    graph: &mut WorkflowGraph,
    hugr: &impl HugrView<Node = N>,
    block_outputs: &HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>>,
    block_preds: &HashMap<N, (NodeIndex, String)>,
) -> Vec<(NodeIndex, String)> {
    match gp {
        GatingPath::Always(n, p) => block_outputs.get(&(*n, *p)).unwrap().clone(),
        GatingPath::Branch(br, opts) => {
            // guppy only produces bools; non-unit-sums have been converted, but
            // TODO deal with >2-way branches
            let tag = block_preds.get(br).unwrap();
            if let [Some(fal), Some(tr)] = &opts[..]
                && predicate_is_bool(&hugr.get_optype(*br).as_dataflow_block().unwrap().sum_rows)
            {
                let fal = build_inputs(fal, graph, hugr, block_outputs, block_preds);
                let tr = build_inputs(tr, graph, hugr, block_outputs, block_preds);
                return build_if_else(graph, tag, tr, fal);
            }

            let present_opts = opts
                .iter()
                .enumerate()
                .filter_map(|(i, opt)| {
                    opt.as_ref().map(|path| {
                        (
                            i,
                            build_inputs(path, graph, hugr, block_outputs, block_preds),
                        )
                    })
                })
                .collect::<Vec<_>>();
            build_if_else_tree(graph, tag, &present_opts)
        }
    }
}

fn build_if_else_tree(
    graph: &mut WorkflowGraph,
    tag: &(NodeIndex, String),
    tagged_rows: &[(usize, Vec<(NodeIndex, String)>)],
) -> Vec<(NodeIndex, String)> {
    // we expect present_opts to be sorted.
    if let [(_, vals)] = tagged_rows {
        return vals.clone();
    }
    let (left, right) = tagged_rows.split_at(tagged_rows.len() / 2);
    let op = if right.len() > 1 { "igte_s" } else { "eq" };
    let cond = task(graph, "inmemory", op, ["a", "b"], ["value"]);
    let (cst, p) = constant(graph, right[0].0).unwrap();
    let cond_inputs = [tag.clone(), (cst, p.to_string())];
    wire_up(graph, &cond_inputs, cond, ["a", "b"]);

    let tr = build_if_else_tree(graph, tag, right);
    let fal = build_if_else_tree(graph, tag, left);
    build_if_else(graph, &(cond, "value".to_string()), tr, fal)
}

fn build_if_else(
    graph: &mut WorkflowGraph,
    pred: &(NodeIndex, String),
    tr: Vec<(NodeIndex, String)>,
    fal: Vec<(NodeIndex, String)>,
) -> Vec<(NodeIndex, String)> {
    tr.iter()
        .zip_eq(&fal)
        .map(|(t, f)| {
            if t == f {
                t.clone()
            } else {
                let node = if_else(graph);
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

fn leaves<N: HugrNode>(
    gp: Option<&GatingPath<N>>,
) -> impl Iterator<Item = ((N, OutgoingPort), GatingPath<N>)> {
    gp.into_iter().flat_map(|gp| gp.leaves())
}
