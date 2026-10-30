//! Functions for converting a [Hugr] into a [WorkflowGraph]
use std::collections::HashMap;
use std::iter::once;

use super::{NodeDefinition, WorkflowGraph};
use hugr::core::HugrNode;
use hugr::extension::prelude::{ConstUsize, TupleOpDef};
use hugr::ops::{DataflowOpTrait as _, ExtensionOp, OpType, Tag, TailLoop, Value, constant::Sum};
use hugr::std_extensions::arithmetic::{float_types::ConstF64, int_types::ConstInt};
use hugr::std_extensions::collections::list::ListValue;
use hugr::types::{Signature, SumType};
use hugr::{Hugr, HugrView, PortIndex as _};
use itertools::Itertools as _;
use miette::{IntoDiagnostic, Report};
use petgraph::visit::{Topo, Walker};
use portgraph::NodeIndex;
use serde_json::Number;

mod cfg;
use cfg::convert_cfg;
use tket::passes::ComposablePass;
use tket::passes::cfg::remove_cycles::RemoveCFGCyclesPass;

struct GraphWithFuncs<N: HugrNode> {
    graph: WorkflowGraph,
    funcs: HashMap<N, NodeIndex>,
}

impl<N: HugrNode> GraphWithFuncs<N> {
    fn new(num_inputs: usize, num_outputs: usize) -> (Self, Vec<(NodeIndex, String)>) {
        Self::new_with_names(
            (0..num_inputs).map(|i| format!("in{i}")),
            (0..num_outputs).map(|i| format!("out{i}")),
        )
    }

    fn new_with_names(
        input_names: impl IntoIterator<Item = String>,
        output_names: impl IntoIterator<Item = String>,
    ) -> (Self, Vec<(NodeIndex, String)>) {
        let mut graph = WorkflowGraph::new(output_names);
        let input_results = input_names
            .into_iter()
            .map(|name| {
                let n = graph.add_node(
                    NodeDefinition::Input { name: name.clone() },
                    vec![],
                    vec![name.clone()],
                );
                (n, name)
            })
            .collect::<Vec<_>>();
        (
            Self {
                graph,
                funcs: HashMap::new(),
            },
            input_results,
        )
    }

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
        let new_node = self
            .graph
            .add_node(graph_const(wg)?, vec![], vec!["value".to_string()]);
        self.funcs.insert(node, new_node);
        Ok(new_node)
    }

    fn add_node<INP: ToString, OUT: ToString, const N_OUTS: usize>(
        &mut self,
        def: NodeDefinition,
        inputs: impl IntoIterator<Item = (INP, (NodeIndex, String))>,
        outputs: [OUT; N_OUTS],
    ) -> [(NodeIndex, String); N_OUTS] {
        let (inport_names, inputs): (Vec<String>, Vec<(NodeIndex, String)>) = inputs
            .into_iter()
            .map(|(pn, val)| (pn.to_string(), val))
            .unzip();
        let outport_names = outputs.map(|o| o.to_string());
        let new_node =
            self.graph
                .add_node(def, inport_names.clone(), outport_names.iter().cloned());
        wire_up(&mut self.graph, &inputs, new_node, &inport_names);
        outport_names.map(|o| (new_node, o))
    }
}

fn graph_const(graph: WorkflowGraph) -> miette::Result<NodeDefinition> {
    Ok(NodeDefinition::Const {
        value: serde_json::to_value(graph).into_diagnostic()?,
    })
}

fn wire_up<'a, T: AsRef<str> + 'a + ?Sized>(
    graph: &'a mut WorkflowGraph,
    inputs: impl IntoIterator<Item = &'a (NodeIndex, String)>,
    tgt_node: NodeIndex,
    tgt_ports: impl IntoIterator<Item = &'a T>,
) {
    for ((src_node, src_port), tgt_port) in inputs.into_iter().zip_eq(tgt_ports) {
        graph
            .link_nodes_by_port_name(*src_node, src_port, tgt_node, tgt_port.as_ref())
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
        OpType::TailLoop(_) => convert_tail_loop(hugr, node, graph, inputs),
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

            let ins = std::iter::once("graph".to_string())
                .chain(ins.map(|(p, _)| {
                    debug_assert!(p.index() > 0);
                    format!("in{}", p.index() - 1)
                }))
                .collect::<Vec<_>>();
            let outs = hugr
                .out_value_types(node)
                .map(|(p, _)| format!("out{}", p.index()))
                .collect::<Vec<_>>();
            let ni = graph
                .graph
                .add_node(NodeDefinition::Eval {}, ins.clone(), outs.clone());
            wire_up(&mut graph.graph, &inputs, ni, &ins);
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
                Some("graph".to_string()).into_iter().chain(args_in.clone()),
                outs.clone(),
            );
            graph
                .graph
                .link_nodes_by_port_name(func_node, "value", ni, "graph")?;
            wire_up(&mut graph.graph, &inputs, ni, &args_in);
            return Ok(outs.into_iter().map(|port| (ni, port)).collect());
        }
        OpType::LoadFunction(_) => {
            let func = hugr.static_source(node).ok_or_else(|| {
                miette::miette!("LoadFunction {node} did not have a static source")
            })?;
            let func_node = graph.get_func_const(hugr, func)?;
            return Ok(vec![(func_node, "value".to_string())]);
        }
        OpType::Tag(Tag { tag, variants, .. }) => {
            if variants.len() == 1 {
                assert_eq!(inputs.len(), variants[0].len());
                return Ok(vec![make_tuple(graph, &inputs)]);
            }
            if variants.len() == 2 && variants[0].is_empty() && variants[1].is_empty() {
                // boolean type represented as two empty variants
                let node = graph.graph.add_node(
                    NodeDefinition::Const {
                        value: serde_json::Value::Bool(*tag == 1),
                    },
                    vec![],
                    vec!["value".to_string()],
                );
                return Ok(vec![(node, "value".to_string())]);
            }
            let [tag] = graph.add_node::<String, _, _>(
                NodeDefinition::Const {
                    value: serde_json::Value::Number(serde_json::Number::from(*tag)),
                },
                vec![],
                ["value"],
            );
            let contents = make_tuple(graph, &inputs);
            Ok(vec![make_tuple(graph, &[tag, contents])])
        }
        other => todo!("{other:?}"),
    }
}

fn convert_tail_loop<H: HugrView>(
    hugr: &H,
    node: H::Node,
    graph: &mut GraphWithFuncs<H::Node>,
    inputs: Vec<(NodeIndex, String)>,
) -> miette::Result<Vec<(NodeIndex, String)>> {
    // Implement the conversion of TailLoop here
    let TailLoop {
        just_inputs,
        just_outputs,
        rest,
    } = hugr.get_optype(node).as_tail_loop().unwrap();
    // Body of loop (below) will produce a sum value plus `rest`.`
    // We'll break the tag off and use for the Tierkreis "should_continue" result of the body graph.
    // The tuple of just_inputs / just_outputs will form the first Tierkreis port, followed by `rest`.
    let rest_names = rest
        .iter()
        .enumerate()
        .map(|(i, _)| format!("rest{i}"))
        .collect::<Vec<_>>();
    let inout_names = once("inout_tuple".to_string())
        .chain(rest_names.clone())
        .collect::<Vec<_>>();
    let body_graph = {
        let predicate_is_bool = just_inputs.len() + just_outputs.len() == 0;
        let (mut g, mut inps) = GraphWithFuncs::new_with_names(
            inout_names.clone(),
            once("should_continue".to_string()).chain(inout_names.clone()),
        );
        let mut just_inputs = unpack_tuple(&mut g.graph, inps.remove(0), just_inputs.len());
        just_inputs.extend(inps);
        let mut outs = convert_dfg(hugr, node, &mut g, just_inputs)?;
        // First value delivered to Hugr Output node is a sum whose tag tells us which way to branch
        let (tag, continue_val, just_inouts) = if predicate_is_bool {
            let bool_value = outs.remove(0);
            (
                bool_value.clone(),
                serde_json::Value::Bool(TailLoop::CONTINUE_TAG == 1),
                bool_value,
            )
        } else {
            let [tag, just_inouts] = unpack_pair(&mut g.graph, outs.remove(0));
            (
                tag,
                serde_json::Value::Number(Number::from(TailLoop::CONTINUE_TAG)),
                just_inouts,
            )
        };
        let [continue_const] = g.add_node::<String, _, _>(
            NodeDefinition::Const {
                value: continue_val,
            },
            [],
            ["value"],
        );
        let [should_continue] = g.add_node(
            NodeDefinition::Task {
                worker_name: "builtins".to_string(),
                task_name: "eq".to_string(),
            },
            [("a", tag), ("b", continue_const)],
            ["value"],
        );
        g.graph.link_nodes_by_port_name(
            should_continue.0,
            &should_continue.1,
            g.graph.output_idx(),
            "should_continue",
        )?;
        for ((src_node, src_port), output_name) in once(just_inouts).chain(outs).zip(&inout_names) {
            g.graph.link_nodes_by_port_name(
                src_node,
                &src_port,
                g.graph.output_idx(),
                output_name,
            )?;
        }

        let [g] = graph.add_node::<String, _, _>(graph_const(g.graph)?, [], ["value"]);
        g
    };
    // Now the loop node. To begin the loop, we must first prepare the initial values to fit
    // this format expected by the body, i.e. make a Tierkreis tuple of just_inputs.
    let (just_input_vals, rest_inputs) = inputs.split_at(just_inputs.len());
    let just_inputs_tuple = make_tuple(graph, just_input_vals);
    let loop_in_names = once("graph".to_string())
        .chain(inout_names.clone())
        .collect::<Vec<_>>();
    let loop_node =
        graph
            .graph
            .add_node(NodeDefinition::Loop {}, loop_in_names.clone(), inout_names);
    wire_up(
        &mut graph.graph,
        [body_graph, just_inputs_tuple].iter().chain(rest_inputs),
        loop_node,
        &loop_in_names,
    );

    // Finally unpack the tuple (body of hugr Sum type if of BREAK variant) output from the loop
    Ok(unpack_tuple(
        &mut graph.graph,
        (loop_node, "inout_tuple".to_string()),
        just_outputs.len(),
    )
    .into_iter()
    .chain(rest_names.into_iter().map(|n| (loop_node, n)))
    .collect())
}

fn make_tuple<N: HugrNode>(
    graph: &mut GraphWithFuncs<N>,
    inputs: &[(NodeIndex, String)],
) -> (NodeIndex, String) {
    let [np] = match inputs {
        [] => graph.add_node::<String, _, _>(unit_const(), vec![], ["value"]),
        [inp] => [inp.clone()],
        _ => {
            let rest = make_tuple(graph, &inputs[1..]);
            graph.add_node(
                NodeDefinition::Task {
                    worker_name: "builtins".to_string(),
                    task_name: "tuple".to_string(),
                },
                vec![("a", inputs[0].clone()), ("b", rest)],
                ["value"],
            )
        }
    };
    np
}

fn unpack_pair(graph: &mut WorkflowGraph, input: (NodeIndex, String)) -> [(NodeIndex, String); 2] {
    unpack_tuple(graph, input, 2).try_into().unwrap()
}

fn unpack_tuple(
    graph: &mut WorkflowGraph,
    input: (NodeIndex, String),
    num_elements: usize,
) -> Vec<(NodeIndex, String)> {
    match num_elements {
        0 => vec![],
        1 => vec![input],
        _ => {
            let mut elements = unpack_tuple(graph, input, num_elements - 1);
            let untuple = graph.add_node(
                NodeDefinition::Task {
                    worker_name: "builtins".to_string(),
                    task_name: "untuple".to_string(),
                },
                ["value".to_string()],
                ["a".to_string(), "b".to_string()],
            );
            wire_up(graph, &[elements.pop().unwrap()], untuple, &["value"]);
            elements.push((untuple, "a".to_string()));
            elements.push((untuple, "b".to_string()));
            elements
        }
    }
}

fn unit_const() -> NodeDefinition {
    // unit type, no data. Could represent as empty list?
    NodeDefinition::Const {
        value: serde_json::Value::Null,
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
            if let Some(c) = v.downcast_ref::<ListValue>() {
                return Ok(serde_json::Value::Array(
                    c.get_contents()
                        .iter()
                        .map(const_val)
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            Err(miette::miette!("Unsupported constant value type {v:?}"))
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
    wire_up(graph, &inputs, new_node, &inports);

    Ok(outports.into_iter().map(|port| (new_node, port)).collect())
}

fn lookup_ext_op(eop: &ExtensionOp) -> miette::Result<(NodeDefinition, Vec<String>, Vec<String>)> {
    let num_inputs = eop.signature().input().len();
    let num_outputs = eop.signature().output().len();
    use hugr::extension::simple_op::MakeOpDef as _;
    if let Ok(top) = TupleOpDef::from_op(eop) {
        return match top {
            TupleOpDef::MakeTuple if num_inputs == 0 => {
                Ok((unit_const(), vec![], vec!["value".to_string()]))
            }
            TupleOpDef::MakeTuple if num_inputs == 2 => Ok((
                NodeDefinition::Task {
                    worker_name: "builtins".to_string(),
                    task_name: "tuple".to_string(),
                },
                vec!["a".to_string(), "b".to_string()],
                vec!["value".to_string()],
            )),
            TupleOpDef::UnpackTuple if num_outputs == 2 => Ok((
                NodeDefinition::Task {
                    worker_name: "builtins".to_string(),
                    task_name: "untuple".to_string(),
                },
                vec!["value".to_string()],
                vec!["a".to_string(), "b".to_string()],
            )),
            _ => Err(miette::miette!(
                "Unsupported tuple op with {} inputs and {} outputs",
                num_inputs,
                num_outputs
            )),
        };
    }
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
    GraphWithFuncs::new(sig.input_count(), sig.output_count())
}

impl TryFrom<Hugr> for WorkflowGraph {
    type Error = Report;

    fn try_from(mut hugr: Hugr) -> miette::Result<Self> {
        RemoveCFGCyclesPass::default()
            .run(&mut hugr)
            .map_err(|e| miette::miette!("Failed to remove CFG cycles: {e}"))?;
        graph_from_hugr(&hugr, hugr.entrypoint())
    }
}

fn graph_from_hugr<H: HugrView>(hugr: &H, parent: H::Node) -> miette::Result<WorkflowGraph> {
    let (graph, outputs) = match hugr.get_optype(parent) {
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
    wire_up(&mut graph, &outputs, o, &output_names);
    Ok(graph)
}

#[cfg(test)]
pub(crate) mod test {
    use hugr::hugr::hugrmut::HugrMut;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;

    use hugr::ops::{DFG, DataflowParent, Input, Output, Tag, TailLoop};
    use hugr::std_extensions::arithmetic::{float_types, int_types};
    use hugr::types::Signature;
    use hugr::{Hugr, HugrView, Node};

    use super::{GraphWithFuncs, NodeDefinition, WorkflowGraph, make_tuple, unpack_tuple};

    #[test]
    fn convert_map() {
        let f = File::open("../tierkreis_map.hugr").unwrap();
        let hugr = Hugr::load(BufReader::new(f), None).unwrap();
        WorkflowGraph::try_from(hugr).unwrap();
    }

    #[test]
    fn convert_map_minopt() {
        let f = File::open("../tierkreis_map_minopt.hugr").unwrap();
        let hugr = Hugr::load(BufReader::new(f), None).unwrap();
        WorkflowGraph::try_from(hugr).unwrap();
    }

    pub fn hugr_tuple1() -> WorkflowGraph {
        let mut wg = WorkflowGraph::new(["x", "y", "z"].map(ToString::to_string));
        let [a, b, c] = ["a", "b", "c"].map(ToString::to_string).map(|n| {
            (
                wg.add_node(NodeDefinition::Input { name: n.clone() }, [], [n.clone()]),
                n,
            )
        });
        let mut gwf = GraphWithFuncs::<Node> {
            graph: wg,
            funcs: HashMap::new(),
        };
        let tup = make_tuple(&mut gwf, &[a, b, c]);
        let mut wg = gwf.graph;
        let [a, b, c] = unpack_tuple(&mut wg, tup, 3).try_into().unwrap();
        let ou = wg.output_idx();
        wg.link_nodes_by_port_name(a.0, &a.1, ou, "x").unwrap();
        wg.link_nodes_by_port_name(b.0, &b.1, ou, "y").unwrap();
        wg.link_nodes_by_port_name(c.0, &c.1, ou, "z").unwrap();
        wg
    }

    pub fn never_loop() -> WorkflowGraph {
        let int_ty = || int_types::INT_TYPES[6].to_owned();
        let mut h = Hugr::new_with_entrypoint(DFG {
            signature: Signature::new_endo([int_ty()]),
        })
        .unwrap();
        let ep = h.entrypoint();
        let inp = h.add_node_with_parent(
            ep,
            Input {
                types: [int_ty()].into(),
            },
        );
        let out = h.add_node_with_parent(
            ep,
            Output {
                types: [int_ty()].into(),
            },
        );
        let tl = TailLoop {
            just_inputs: [].into(),
            just_outputs: [].into(),
            rest: [int_ty()].into(),
        };
        let Signature { input, output } = tl.inner_signature().into_owned();
        eprintln!("TAILLOOP SIGNATURE: {}   -->   {}", input, output);
        let loop_ = h.add_node_with_parent(ep, tl);
        h.connect(inp, 0, loop_, 0);
        h.connect(loop_, 0, out, 0);
        let inp = h.add_node_with_parent(loop_, Input { types: input });
        let out = h.add_node_with_parent(loop_, Output { types: output });
        let tag = h.add_node_with_parent(loop_, Tag::new(TailLoop::BREAK_TAG, vec![[].into(); 2]));
        h.connect(tag, 0, out, 0);
        h.connect(inp, 0, out, 1);
        eprintln!("{}", h.mermaid_string());
        h.validate().unwrap();
        h.try_into().unwrap()
    }

    pub fn never_loop2() -> WorkflowGraph {
        let int_ty = || int_types::INT_TYPES[6].to_owned();
        use hugr::std_extensions::arithmetic::float_types::float64_type;
        let mut h = Hugr::new_with_entrypoint(DFG {
            signature: Signature::new_endo([int_ty(), float64_type()]),
        })
        .unwrap();
        let ep = h.entrypoint();
        let inp = h.add_node_with_parent(
            ep,
            Input {
                types: [int_ty(), float64_type()].into(),
            },
        );
        let out = h.add_node_with_parent(
            ep,
            Output {
                types: [int_ty(), float64_type()].into(),
            },
        );
        let tl = TailLoop {
            just_inputs: [int_ty(), float64_type()].into(),
            just_outputs: [int_ty(), float64_type()].into(),
            rest: [].into(),
        };
        let Signature { input, output } = tl.inner_signature().into_owned();
        //eprintln!("TAILLOOP SIGNATURE: {}   -->   {}", input, output);
        let loop_ = h.add_node_with_parent(ep, tl);
        for i in 0..=1 {
            h.connect(inp, i, loop_, i);
            h.connect(loop_, i, out, i);
        }
        let inp = h.add_node_with_parent(loop_, Input { types: input });
        let out = h.add_node_with_parent(loop_, Output { types: output });
        let tag = h.add_node_with_parent(loop_, Tag::new(TailLoop::BREAK_TAG, vec![[int_ty(), float64_type()].into(); 2]));
        h.connect(tag, 0, out, 0);
        h.connect(inp, 0, tag, 0);
        h.connect(inp, 1, tag, 1);
        //eprintln!("{}", h.mermaid_string());
        h.validate().unwrap();
        let wg: WorkflowGraph = h.try_into().unwrap();
        wg.print("").unwrap();
        wg
    }
}
