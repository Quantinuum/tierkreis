use std::collections::HashMap;

use super::NodeDefinition;
use super::WorkflowGraph;
use hugr::ops::{DataflowOpTrait as _, ExtensionOp, OpType};
use hugr::{Hugr, HugrView, PortIndex, types::Type};
use miette::Report;
use petgraph::algo::dominators::{self, Dominators};
use petgraph::visit::{Topo, Walker};
use portgraph::NodeIndex;

#[expect(unused)]
fn compute_dominator<H: HugrView>(
    hugr: &H,
    parent: H::Node,
) -> (Dominators<portgraph::NodeIndex>, H::RegionPortgraphNodes) {
    let sg = hugr.scheduling_graph(parent);
    let entry_node = hugr.children(parent).next().unwrap();
    let doms = dominators::simple_fast(sg.petgraph(), sg.node_to_pg(entry_node));
    (doms, sg.into_node_map())
}

fn convert_dataflow_op<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut WorkflowGraph,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    match hugr.get_optype(node) {
        OpType::DFG(_) => convert_dfg(hugr, node, graph, inputs),
        OpType::ExtensionOp(eop) => convert_ext_op(eop, graph, inputs),
        other => todo!("{other:?}"),
    }
}

fn convert_dfg<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut WorkflowGraph,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    let [inp, out] = hugr
        .get_io(node)
        .ok_or_else(|| miette::miette!("DFG node must have IO children"))?;
    // Ignore Order edges...

    let mut node_map = HashMap::from([(inp, inputs)]);

    let sg = hugr.scheduling_graph(node);
    let topo = Topo::new(sg.petgraph());
    for pn in topo.iter(sg.petgraph()) {
        let n = sg.pg_to_node(pn);
        if matches!(hugr.get_optype(n), OpType::Input(_) | OpType::Output(_)) {
            continue;
        }
        let inputs = hugr
            .in_value_types(n)
            .map(|(p, _)| {
                let (src_n, src_p) = hugr.single_linked_output(n, p).unwrap();
                node_map[&src_n][src_p.index()].clone()
            })
            .collect();
        let out_srcs = convert_dataflow_op(hugr, n, graph, inputs)?;
        node_map.insert(n, out_srcs);
    }
    Ok(hugr
        .in_value_types(out)
        .map(|(p, _)| {
            let (src_n, src_p) = hugr.single_linked_output(out, p).unwrap();
            node_map[&src_n][src_p.index()].clone()
        })
        .collect())
}

fn convert_ext_op(
    eop: &ExtensionOp,
    graph: &mut WorkflowGraph,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    // Keep it simple for now, support only hugr ops that become a single node in the workflow graph
    let (node_def, inports, outports) = lookup_ext_op(eop)?;

    let new_node = graph.add_node(node_def, inports.clone(), outports.clone());

    assert_eq!(inputs.len(), inports.len());
    for ((inp_n, inp_p), tgt_port) in inputs.iter().zip(inports) {
        graph
            .link_nodes_by_port_name(*inp_n, inp_p, new_node, &tgt_port)
            .unwrap();
    }

    Ok(outports.into_iter().map(|port| (new_node, port)).collect())
}

fn lookup_ext_op(eop: &ExtensionOp) -> miette::Result<(NodeDefinition, Vec<String>, Vec<String>)> {
    let num_inputs = eop.signature().input().len();
    let num_outputs = eop.signature().output().len();
    if [
        hugr::std_extensions::arithmetic::int_ops::EXTENSION_ID,
        hugr::std_extensions::arithmetic::float_ops::EXTENSION_ID,
        hugr::std_extensions::arithmetic::conversions::EXTENSION_ID,
    ]
    .contains(eop.extension_id())
    {
        // Assume everything works (we might need some renames?)
        // and let the runtime fail if it doesn't.
        static INPUT_PORT_NAMES: [&str; 2] = ["a", "b"];
        assert_eq!(num_inputs, INPUT_PORT_NAMES.len());
        assert_eq!(num_outputs, 1);
        Ok((
            NodeDefinition::Task {
                worker_name: "inmemory".to_string(),
                task_name: eop.def().name().to_string(),
            },
            INPUT_PORT_NAMES
                .iter()
                .take(num_inputs)
                .map(|s| s.to_string())
                .collect(),
            vec!["value".to_string()],
        ))
    } else {
        Err(miette::miette!(
            "Unknown extension op: {} {}",
            eop.def().extension_id(),
            eop.def().name()
        ))
    }
}

fn wrapper_graph(
    inputs: impl IntoIterator<Item = (impl PortIndex, Type)>,
    outputs: impl IntoIterator<Item = (impl PortIndex, Type)>,
) -> (WorkflowGraph, Vec<(NodeIndex, String)>) {
    let mut graph = WorkflowGraph::new(
        outputs
            .into_iter()
            .map(|(p, _)| format!("out{}", p.index())),
    );
    let input_results = inputs
        .into_iter()
        .map(|(p, _)| {
            let name = format!("in{}", p.index());
            let n = graph.add_node(
                NodeDefinition::Input { name: name.clone() },
                vec![],
                vec![name.clone()],
            );
            (n, name)
        })
        .collect::<Vec<_>>();
    (graph, input_results)
}

impl TryFrom<Hugr> for WorkflowGraph {
    type Error = Report;

    fn try_from(hugr: Hugr) -> miette::Result<Self> {
        let entrypoint = hugr.entrypoint();
        let (mut graph, outputs) = match hugr.entrypoint_optype() {
            OpType::FuncDefn(_) => {
                // Not supported by convert_dataflow_op as not a DataflowOp!
                let [inp, out] = hugr
                    .get_io(entrypoint)
                    .ok_or_else(|| miette::miette!("entrypoint must have IO children"))?;
                let (mut graph, inputs) =
                    wrapper_graph(hugr.out_value_types(inp), hugr.in_value_types(out));
                let outputs = convert_dfg(&hugr, entrypoint, &mut graph, inputs)?;
                (graph, outputs)
            }
            OpType::DFG(_) | OpType::CFG(_) => {
                let (mut graph, inputs) = wrapper_graph(
                    hugr.in_value_types(entrypoint),
                    hugr.out_value_types(entrypoint),
                );
                let outputs = convert_dataflow_op(&hugr, entrypoint, &mut graph, inputs)?;
                (graph, outputs)
            }
            // We could support others, but we don't really expect them to occur
            other => panic!("Entrypoint must be FuncDefn, DFG or CFG, got {other:?}"),
        };

        let output_names = graph
            .input_names(graph.output_node)
            .unwrap()
            .cloned()
            .collect::<Vec<_>>();
        for ((src_node, src_port), port) in outputs.into_iter().zip(output_names) {
            graph.link_nodes_by_port_name(src_node, &src_port, graph.output_node, &port)?;
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod test {
    use hugr::Hugr;
    use std::fs::File;
    use std::io::BufReader;
    #[test]
    fn test_convert_simple_arith() {
        let f = File::open("../simple_arith.hugr").unwrap();
        let hugr = Hugr::load(BufReader::new(f), None).unwrap();
        super::WorkflowGraph::try_from(hugr).unwrap();
    }
}
