import { Link } from "@tanstack/react-router";
import { Minus, Plus } from "lucide-react";

type SearchData = {
  openEvals?: string[] | undefined;
  openLoops?: string[] | undefined;
  openMaps?: string[] | undefined;
};

export const ZoomInButton = (props: {
  wid: string;
  loc: string;
  node_loc: string;
  node_type: "eval" | "loop" | "map";
}) => {
  const { wid, loc, node_loc, node_type } = props;

  const params = { wid: wid, loc };
  const search = (prev: SearchData): SearchData => {
    const openEvals = [...(prev.openEvals ?? [])];
    const openLoops = [...(prev.openLoops ?? [])];
    const openMaps = [...(prev.openMaps ?? [])];

    if (node_type == "eval") openEvals.push(node_loc);
    else if (node_type == "loop") openLoops.push(node_loc);
    else if (node_type == "map") openMaps.push(node_loc);
    else node_type satisfies never;

    return { openEvals, openLoops, openMaps };
  };
  return (
    <Link
      to="/workflows/$wid/nodes/$loc"
      params={params}
      search={(p) => search(p)}
    >
      <Plus />
    </Link>
  );
};

export const zoomOutButton = (wid: string, loc: string, node_loc: string) => {
  const params = { wid, loc };
  const search = (prev: SearchData): SearchData => {
    const openEvals = prev.openEvals?.filter((x) => !x.startsWith(node_loc));
    const openLoops = prev.openLoops?.filter((x) => !x.startsWith(node_loc));
    const openMaps = prev.openMaps?.filter((x) => !x.startsWith(node_loc));
    return { openEvals, openLoops, openMaps };
  };
  return (
    <Link to="/workflows/$wid/nodes/$loc" params={params} search={search}>
      <Minus style={{ width: 48, height: 48 }} />
    </Link>
  );
};
