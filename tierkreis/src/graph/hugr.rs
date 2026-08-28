use std::collections::HashMap;

use super::NodeDefinition;
use super::WorkflowGraph;
use hugr::{Hugr, HugrView, PortIndex as _, ops::OpType};
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
        other => todo!("{other:?}"),
    }
}

fn graph_for_node<H: HugrView>(
    hugr: &H,
    node: H::Node,
) -> (WorkflowGraph, Vec<(NodeIndex, String)>) {
    let mut graph = WorkflowGraph::new(
        hugr.out_value_types(node)
            .map(|(p, _)| format!("out{}", p.index())),
    );
    let input_results = hugr
        .in_value_types(node)
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

    let (mut graph, inps) = graph_for_node(hugr, node);

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

impl TryFrom<Hugr> for WorkflowGraph {
    type Error = Report;

    fn try_from(hugr: Hugr) -> miette::Result<Self> {
        convert_node(&hugr, hugr.entrypoint())
    }
}
