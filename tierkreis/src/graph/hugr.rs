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

fn convert_node<H: HugrView>(hugr: &H, node: H::Node) -> miette::Result<WorkflowGraph> {
    match hugr.get_optype(node) {
        OpType::DFG(_) => convert_dfg(hugr, node),
        OpType::ExtensionOp(eop) => convert_ext_op(hugr, node, eop),
        other => todo!("{other:?}"),
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

fn convert_dfg<H: HugrView>(hugr: &H, node: H::Node) -> miette::Result<WorkflowGraph> {
    let [inp, out] = hugr
        .get_io(node)
        .ok_or_else(|| miette::miette!("DFG node must have IO children"))?;
    // Ignore Order edges...

    let (mut graph, inps) = wrapper_graph(hugr.out_value_types(inp), hugr.in_value_types(out));

    let mut node_map = HashMap::from([(inp, inps)]);

    let sg = hugr.scheduling_graph(node);
    let topo = Topo::new(sg.petgraph());
    for pn in topo.iter(sg.petgraph()) {
        let n = sg.pg_to_node(pn);
        if matches!(hugr.get_optype(n), OpType::Input(_) | OpType::Output(_)) {
            continue;
        }
        let child_graph = convert_node(hugr, n)?;
        let inputs = hugr
            .in_value_types(n)
            .map(|(p, _)| {
                let name = format!("in{}", p.index());
                let (src_n, src_p) = hugr.single_linked_output(n, p).unwrap();
                let outport = node_map[&src_n][src_p.index()].clone();
                (name, outport)
            })
            .collect::<HashMap<String, (portgraph::NodeIndex, String)>>();
        let (_, mut outs) = graph.insert_graph(child_graph, inputs);
        let out_srcs = hugr
            .out_value_types(n)
            .map(|(p, _)| {
                let name = format!("out{}", p.index());
                outs.remove(&name).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(outs.is_empty());
        node_map.insert(n, out_srcs);
    }
    for (p, _) in hugr.in_value_types(out) {
        let (src_n, src_p) = hugr.single_linked_output(out, p).unwrap();
        let (src_n, ref src_p) = node_map[&src_n][src_p.index()];
        graph
            .link_nodes_by_port_name(
                src_n,
                src_p,
                graph.output_node,
                &format!("out{}", p.index()),
            )
            .unwrap();
    }
    Ok(graph)
}

fn convert_ext_op<H: HugrView>(
    hugr: &H,
    node: H::Node,
    eop: &ExtensionOp,
) -> miette::Result<WorkflowGraph> {
    let (mut graph, inps) = wrapper_graph(hugr.in_value_types(node), hugr.out_value_types(node));
    // Keep it simple for now, support only hugr ops that become a single node in the workflow graph
    let (node_def, inports, outports) = lookup_ext_op(eop)?;

    let new_node = graph.add_node(node_def, inports.clone(), outports.clone());

    assert_eq!(inps.len(), inports.len());
    for ((inp_n, inp_p), tgt_port) in inps.iter().zip(inports) {
        graph
            .link_nodes_by_port_name(*inp_n, inp_p, new_node, &tgt_port)
            .unwrap();
    }

    let graph_outputs = graph
        .input_names(graph.output_node)
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(graph_outputs.len(), outports.len());
    for (src_port, tgt_port) in outports.iter().zip(graph_outputs) {
        graph
            .link_nodes_by_port_name(new_node, src_port, graph.output_node, &tgt_port)
            .unwrap();
    }
    Ok(graph)
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

impl TryFrom<Hugr> for WorkflowGraph {
    type Error = Report;

    fn try_from(hugr: Hugr) -> miette::Result<Self> {
        convert_node(&hugr, hugr.entrypoint())
    }
}
