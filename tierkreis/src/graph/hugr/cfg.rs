use std::collections::{HashMap, hash_map::Entry};

use itertools::Itertools;

use hugr::core::HugrNode;
use hugr::{HugrView, OutgoingPort};
use hugr_core::hugr::internal::PortgraphNodeMap;
use petgraph::algo::dominators::{self, Dominators};
use portgraph::NodeIndex;

use super::GraphWithFuncs;

struct DomTreeNode<N> {
    node: N,
    // In topsort order (child before any sibling it can reach)
    children: Vec<(GatingPath<N>, DomTreeNode<N>)>,
    exit_edges: Vec<(GatingPath<N>, N, OutgoingPort, N)>,
    loop_backedges: Vec<(GatingPath<N>, N, OutgoingPort)>,
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
        let mut exit_edges = Vec::new();
        let mut loop_backedges = Vec::new();

        let mut child_paths = HashMap::new();
        // Process edges from this node (perhaps a loop header)
        for outport in hugr.node_outputs(n.into()) {
            let path = if hugr.node_outputs(n.into()).skip(1).next().is_some() {
                GatingPath::branch(n, outport)
            } else {
                GatingPath::Always
            };
            // Çontrol Flow outports should have exactly one outgoing edge
            let (tgt, _) = hugr
                .linked_inputs(n.into(), outport)
                .exactly_one()
                .ok()
                .unwrap();
            if children_by_bb.contains_key(&tgt) {
                child_paths
                    .entry(tgt)
                    .or_insert(GatingPath::never(n))
                    .union(&path);
            } else if tgt == n {
                loop_backedges.push((path, n, outport));
            } else {
                exit_edges.push((path, n, outport, tgt));
            }
        }
        // We want to process children in reverse topsort order: any child C1 with an exit edge to C2, must be processed *before* C2.
        let mut ordered_children = Vec::new();
        fn rev_sort<N: HugrNode>(
            ordered: &mut Vec<N>,
            child: N,
            remaining_children: &mut HashMap<N, &DomTreeNode<N>>,
        ) {
            let Some(dtn) = remaining_children.get(&child) else {
                return;
            };
            // Targets of exit edges pushed onto <ordered> first
            for (_, _, _, succ) in &dtn.exit_edges {
                rev_sort(ordered, *succ, remaining_children);
            }
            ordered.push(child);
        }
        let mut remaining_children = children_by_bb.iter().map(|(n, c)| (*n, c)).collect();
        for child in children_by_bb.values() {
            rev_sort(&mut ordered_children, child.node, &mut remaining_children);
        }
        // This means targets of exit edges will be processed *after* all the sources of said exit edges:
        ordered_children.reverse();

        // Now process children in the determined order
        let mut children = Vec::new();
        let mut children_by_bb = children_by_bb;
        for child in ordered_children {
            let path_to_child = child_paths.remove(&child).unwrap();
            let child_dtn = children_by_bb.remove(&child).unwrap();
            for (path_from_child_to_exit, src, idx, dst) in &child_dtn.exit_edges {
                let path_to_exit = path_to_child.concat(path_from_child_to_exit);
                assert!(
                    doms.dominators(ni).unwrap().contains(
                        &doms
                            .immediate_dominator(node_map.to_portgraph(*dst))
                            .unwrap()
                    )
                );
                if children_by_bb.contains_key(dst) {
                    child_paths
                        .entry(*dst)
                        .or_insert(GatingPath::never(n))
                        .union(&path_to_exit);
                } else if *dst == n {
                    loop_backedges.push((path_to_exit, *src, *idx));
                } else {
                    exit_edges.push((path_to_exit, *src, *idx, *dst));
                }
            }
            children.push((path_to_child, child_dtn))
        }
        DomTreeNode {
            node: n,
            children,
            exit_edges,
            loop_backedges,
        }
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
    build_dom_tree(hugr, node);
    Err(miette::miette!(
        "CFG conversion not yet implemented for Hugr node {:?}",
        node
    ))
}

#[derive(Clone, Debug)]
enum GatingPath<N> {
    Always,
    Branch(N, HashMap<OutgoingPort, GatingPath<N>>),
}

impl<N: HugrNode> GatingPath<N> {
    fn never(node: N) -> Self {
        GatingPath::Branch(node, HashMap::new())
    }

    fn branch(node: N, port: OutgoingPort) -> Self {
        GatingPath::Branch(node, HashMap::from([(port, GatingPath::Always)]))
    }

    fn concat(&self, other: &GatingPath<N>) -> Self {
        match self {
            GatingPath::Always => other.clone(),
            GatingPath::Branch(node, map) => GatingPath::Branch(
                *node,
                map.into_iter()
                    .map(|(k, v)| (*k, v.concat(other)))
                    .collect(),
            ),
        }
    }

    fn union(&mut self, other: &GatingPath<N>) {
        let GatingPath::Branch(n, map) = self else {
            panic!("Union of Always with {other:?}");
        };
        if let GatingPath::Branch(n2, map2) = other
            && n == n2
        {
            for (k, v) in map2 {
                match map.entry(*k) {
                    Entry::Vacant(ve) => {
                        ve.insert(v.clone());
                    }
                    Entry::Occupied(mut e) => e.get_mut().union(v),
                }
            }
        } else {
            panic!("Cannot union Branch({n:?}) with {other:?}");
        }
    }
}
