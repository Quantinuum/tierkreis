use itertools::Itertools;
use std::collections::HashMap;

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort};
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

    // values delivered to control-flow outports of various descendants
    let mut block_outputs: HashMap<(N, OutgoingPort), Vec<(NodeIndex, String)>> = HashMap::new();

    // Compile body. (Easy - the complexity of this function is all about the branches!)
    let (predicate, this_block_outs) = {
        let mut all_outs = convert_dfg(hugr, dtn.node, graph, this_block_inputs.clone())?;
        (all_outs.remove(0), all_outs)
    };
    // Edges from "root"
    assert_eq!(hugr.node_outputs(dtn.node).count(), bb.sum_rows.len());
    // Generate outputs of BB for each successor (each gets different unpacking of Sum).
    // The code here is intended to handle arity > 2 but TODO we need to build conditional
    // predicates differently for arity > 2, probably by converting to a chain of binary tests.
    assert!([1, 2].contains(&bb.sum_rows.len()));
    if bb.sum_rows.len() == 1 {
        let mut outs = unpack_tuple(&mut graph.graph, predicate, bb.sum_rows[0].len());
        outs.extend(this_block_outs);
        block_outputs.insert((dtn.node, OutgoingPort::from(0)), outs);
    } else if bb.sum_rows.iter().all(|row| row.is_empty()) {
        block_preds.insert(dtn.node, predicate);
        for p in hugr.node_outputs(dtn.node) {
            block_outputs.insert((dtn.node, p), this_block_outs.clone());
        }
    } else {
        let [tag, mut rest] = unpack_tuple(&mut graph.graph, predicate, 2)
            .try_into()
            .unwrap();
        //tag is 0 or 1, which we need to map to true/false
        let tag_bool = task(&mut graph.graph, "inmemory", "eq", ["a", "b"], ["value"]);
        let (c, p) = constant(&mut graph.graph, 1).unwrap();
        let eq_inputs = [tag, (c, p.to_string())];
        wire_up(&mut graph.graph, &eq_inputs, tag_bool, ["a", "b"]);
        let tag = (tag_bool, "value".to_string());

        block_preds.insert(dtn.node, tag);
        // Unpack the tuple only once for all values of `tag`. If some variants are shorter,
        // corresponding unpacks will be unused - fine under lazy evaluation.
        let max_elems = bb.sum_rows.iter().map(|row| row.len()).max().unwrap_or(0);
        // Build vec where element [0] is unpacking of pair;
        // unpacking of triple is [0].0 followed by [1];
        // unpacking of quadruple is [0].0 followed by [1].0 followed by [2]; etc.
        let do_not_use = (
            NodeIndex::new(NodeIndex::<u32>::max_index()),
            "DO NOT USE".to_string(),
        );
        let mut unpacked = vec![[do_not_use, rest.clone()]];

        for _ in 0..max_elems {
            let elem_rest: [_; 2] = unpack_tuple(&mut graph.graph, rest, 2).try_into().unwrap();
            [_, rest] = elem_rest.clone();
            unpacked.push(elem_rest);
        }
        for (p, pred_row) in hugr.node_outputs(dtn.node).zip(&bb.sum_rows) {
            let outs = (1..pred_row.len())
                .map(|i| &unpacked[i])
                .map(|[e, _]| e.clone())
                .chain((!pred_row.is_empty()).then(|| unpacked[pred_row.len() - 1][1].clone()))
                .collect();
            block_outputs.insert((dtn.node, p), outs);
        }
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
            // guppy only produces bools; non-unit-sums have been converted, but
            assert_eq!(opts.len(), 2); // TODO deal with >2-way branches

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
    }
}

fn leaves<N: HugrNode>(
    gp: Option<&GatingPath<N>>,
) -> impl Iterator<Item = ((N, OutgoingPort), GatingPath<N>)> {
    gp.into_iter().flat_map(|gp| gp.leaves())
}
