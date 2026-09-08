//! Functions for converting a [Hugr] into a [WorkflowGraph]
use std::collections::HashMap;

use super::NodeDefinition;
use super::WorkflowGraph;
use hugr::core::HugrNode;
use hugr::extension::prelude::ConstUsize;
use hugr::ops::{DataflowOpTrait as _, ExtensionOp, OpType, Value, constant::Sum};
use hugr::std_extensions::arithmetic::{float_types::ConstF64, int_types::ConstInt};
use hugr::types::{Signature, SumType};
use hugr::{Hugr, HugrView, PortIndex as _};
use miette::{IntoDiagnostic, Report};
use petgraph::visit::{Topo, Walker};
use portgraph::NodeIndex;
use serde_json::Number;

mod cfg;
use cfg::convert_cfg;

struct GraphWithFuncs<N: HugrNode> {
    graph: WorkflowGraph,
    funcs: HashMap<N, NodeIndex>,
}

impl<N: HugrNode> GraphWithFuncs<N> {
    fn get_func_const(
        &mut self,
        hugr: &impl HugrView<Node = N>,
        node: N,
    ) -> miette::Result<NodeIndex> {
        if let Some(n) = self.funcs.get(&node) {
            return Ok(*n);
        }
        hugr.get_optype(node).as_func_defn().unwrap();
        let wg = graph_from_hugr(hugr, node)?;
        let new_node = self.graph.add_node(
            NodeDefinition::Const {
                value: serde_json::to_value(wg).into_diagnostic()?,
            },
            vec![],
            vec!["value".to_string()],
        );
        self.funcs.insert(node, new_node);
        Ok(new_node)
    }
}

fn wire_up(
    graph: &mut WorkflowGraph,
    inputs: Vec<(NodeIndex, String)>,
    tgt_node: NodeIndex,
    tgt_ports: impl IntoIterator<Item = String>,
) {
    for ((src_node, src_port), tgt_port) in inputs.into_iter().zip(tgt_ports) {
        graph
            .link_nodes_by_port_name(src_node, &src_port, tgt_node, &tgt_port)
            .unwrap();
    }
}

fn convert_dataflow_op<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut GraphWithFuncs<H::Node>,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    match hugr.get_optype(node) {
        OpType::DFG(_) => convert_dfg(hugr, node, graph, inputs),
        OpType::CFG(_) => convert_cfg(hugr, node, graph, inputs),
        OpType::Const(_) => Ok(vec![]), // Ignore on its own, will be converted as part of any LoadConstant
        OpType::LoadConstant(_) => {
            let cst = hugr.static_source(node).ok_or_else(|| {
                miette::miette!("LoadConstant node {node} has no attached constant")
            })?;
            let cst = match hugr.get_optype(cst) {
                OpType::Const(val) => val,
                _ => {
                    return Err(miette::miette!(
                        "Expected constant node for LoadConstant source"
                    ));
                }
            };
            let value = const_val(&cst.value)?;
            Ok(vec![(
                graph.graph.add_node(
                    NodeDefinition::Const { value },
                    vec![],
                    vec!["value".to_string()],
                ),
                "value".to_string(),
            )])
        }
        OpType::ExtensionOp(eop) => convert_ext_op(eop, &mut graph.graph, inputs),
        OpType::CallIndirect(_) => {
            let mut ins = hugr.in_value_types(node);
            let (port, ty) = ins.next().unwrap();
            debug_assert!(port.index() == 0 && matches!(*ty, hugr::types::Term::FunctionType(_)));

            let ins = ins
                .map(|(p, _)| {
                    if p.index() == 0 {
                        "func".to_string()
                    } else {
                        format!("in{}", p.index() - 1)
                    }
                })
                .collect::<Vec<_>>();
            let outs = hugr
                .out_value_types(node)
                .map(|(p, _)| format!("out{}", p.index()))
                .collect::<Vec<_>>();
            let ni = graph
                .graph
                .add_node(NodeDefinition::Eval {}, ins.clone(), outs.clone());
            wire_up(&mut graph.graph, inputs, ni, ins);
            return Ok(outs.into_iter().map(|port| (ni, port)).collect());
        }
        OpType::Call(_) => {
            let func = hugr
                .static_source(node)
                .ok_or_else(|| miette::miette!("Call {node} did not have a static source"))?;
            let func_node = graph.get_func_const(hugr, func)?;
            let args_in = hugr
                .in_value_types(node)
                .map(|(p, _)| format!("in{}", p.index()))
                .collect::<Vec<_>>();
            let outs = hugr
                .out_value_types(node)
                .map(|(p, _)| format!("out{}", p.index()))
                .collect::<Vec<_>>();
            let ni = graph.graph.add_node(
                NodeDefinition::Eval {},
                Some("func".to_string()).into_iter().chain(args_in.clone()),
                outs.clone(),
            );
            graph
                .graph
                .link_nodes_by_port_name(func_node, "value", ni, "func")?;
            wire_up(&mut graph.graph, inputs, ni, args_in);
            return Ok(outs.into_iter().map(|port| (ni, port)).collect());
        }
        OpType::LoadFunction(_) => {
            let func = hugr.static_source(node).ok_or_else(|| {
                miette::miette!("LoadFunction {node} did not have a static source")
            })?;
            let func_node = graph.get_func_const(hugr, func)?;
            return Ok(vec![(func_node, "value".to_string())]);
        }
        other => todo!("{other:?}"),
    }
}

fn const_val(value: &Value) -> miette::Result<serde_json::Value> {
    match value {
        Value::Sum(Sum {
            tag,
            values,
            sum_type,
        }) => {
            if sum_type == &SumType::new_unary(2) {
                return Ok(serde_json::Value::Bool(*tag == 1));
            }
            if sum_type.num_variants() != 1 {
                return Err(miette::miette!(
                    "Constant value has Sum type with multiple variants: {sum_type:?}"
                ));
            }
            Ok(serde_json::json!([values
                .iter()
                .map(const_val)
                .collect::<Result<Vec<_>, _>>()?]))
        }
        Value::Extension { e } => {
            let v = e.value();
            if let Some(c) = v.downcast_ref::<ConstUsize>() {
                return Ok(serde_json::Value::Number(Number::from(c.value())));
            }
            if let Some(c) = v.downcast_ref::<ConstInt>() {
                // No way to tell from the constant, only the operations...
                // (Doesn't fit well with Tierkreis' arbitrary precision model?!)
                return Ok(serde_json::Value::Number(Number::from(c.value_s())));
            }
            if let Some(c) = v.downcast_ref::<ConstF64>() {
                let num = Number::from_f64(c.value()).ok_or_else(|| {
                    miette::miette!("Serde does not support non-finite floats {}", c.value())
                })?;
                return Ok(serde_json::Value::Number(num));
            }
            Err(miette::miette!("Unsupported constant value type"))
        }
    }
}

fn convert_dfg<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut GraphWithFuncs<H::Node>,
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
    wire_up(graph, inputs, new_node, inports);

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

fn wrapper_graph<N: HugrNode>(sig: &Signature) -> (GraphWithFuncs<N>, Vec<(NodeIndex, String)>) {
    let mut graph = GraphWithFuncs {
        graph: WorkflowGraph::new((0..sig.output_count()).map(|i| format!("out{i}"))),
        funcs: HashMap::new(),
    };
    let input_results = (0..sig.input_count())
        .map(|i| {
            let name = format!("in{i}");
            let n = graph.graph.add_node(
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
        graph_from_hugr(&hugr, hugr.entrypoint())
    }
}

fn graph_from_hugr<H: HugrView>(hugr: &H, parent: H::Node) -> miette::Result<WorkflowGraph> {
    let (graph, outputs) = match hugr.entrypoint_optype() {
        OpType::FuncDefn(fd) => {
            // Not supported by convert_dataflow_op as not a DataflowOp!
            let (mut graph, inputs) = wrapper_graph(fd.signature().body());
            let outputs = convert_dfg(hugr, parent, &mut graph, inputs)?;
            (graph, outputs)
        }
        OpType::DFG(_) | OpType::CFG(_) => {
            let (mut graph, inputs) = wrapper_graph(hugr.signature(parent).unwrap().as_ref());
            let outputs = convert_dataflow_op(hugr, parent, &mut graph, inputs)?;
            (graph, outputs)
        }
        // We could support others, but we don't really expect them to occur
        other => panic!("Entrypoint must be FuncDefn, DFG or CFG, got {other:?}"),
    };

    let mut graph = graph.graph;
    let output_names = graph
        .input_names(graph.output_node)
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();
    let o = graph.output_node;
    wire_up(&mut graph, outputs, o, output_names);
    Ok(graph)
}

#[cfg(test)]
pub(crate) mod test {
    use super::WorkflowGraph;
    use hugr::Hugr;
    use rstest::fixture;
    use std::fs::File;
    use std::io::BufReader;

    #[fixture]
    pub(crate) fn simple_arith() -> WorkflowGraph {
        let f = File::open("../simple_arith.hugr").unwrap();
        let hugr = Hugr::load(BufReader::new(f), None).unwrap();
        WorkflowGraph::try_from(hugr).unwrap()
    }
}
