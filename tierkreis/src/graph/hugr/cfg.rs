use itertools::Itertools;

use hugr::{HugrView, PortIndex as _};
use hugr_core::hugr::internal::PortgraphNodeMap;
use petgraph::algo::dominators::{self, Dominators};
use portgraph::NodeIndex;

use super::GraphWithFuncs;

struct DomTreeNode<N> {
    node: N,
    children: Vec<DomTreeNode<N>>,
    exit_edges: Vec<(N, usize, N)>,
    loop_backedges: Vec<(N, usize)>,
}

fn build_dom_tree<H: HugrView>(
    hugr: &H,
    cfg: H::Node
) -> DomTreeNode<H::Node> {
    fn build<H:HugrView>(hugr: &H,doms: &Dominators<NodeIndex>, n: H::Node, node_map: &H::RegionPortgraphNodes) -> DomTreeNode<H::Node> {
        let ni = node_map.to_portgraph(n);
        let children = doms.immediately_dominated_by(ni).map(|c| build(hugr, doms, node_map.from_portgraph(c), node_map))
            .collect::<Vec<_>>();
        let child_bbs = children.iter().map(|c| c.node).collect::<Vec<_>>();
        let mut exit_edges = Vec::new();
        let mut loop_backedges = Vec::new();
        for child in &children {
            for (src, idx, dst) in &child.exit_edges {
                assert!(doms.dominators(ni).unwrap().contains(&doms.immediate_dominator(node_map.to_portgraph(*dst)).unwrap()));
                if !child_bbs.contains(dst) {
                    if *dst == n {
                        loop_backedges.push((*src, *idx));
                    } else {
                        exit_edges.push((*src, *idx, *dst));
                    }
                }
            }
        }
        for outport in hugr.node_outputs(n.into()) {
            for (tgt, _) in hugr.linked_inputs(n.into(), outport) {
                let tgti = node_map.to_portgraph(tgt);
                assert!(doms.dominators(ni).unwrap().contains(&doms.immediate_dominator(tgti).unwrap()));
                if tgt == n {
                    loop_backedges.push((n, outport.index()));
                } else if doms.immediate_dominator(tgti) != Some(ni) {
                    exit_edges.push((n, outport.index(), tgt));
                }
            }
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