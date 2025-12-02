import { Link } from "@tanstack/react-router";
import { Minus, Plus } from "lucide-react";

export const openLink = (wid: string, loc: string, node_loc: string) => {
  const params = { wid: wid, loc };
  const search = (prev: { openEvals?: string[] | undefined }) => {
    const openEvals = [node_loc, ...(prev.openEvals ?? [])];
    return { openEvals };
  };
  return (
    <Link to="/workflows/$wid/nodes/$loc" params={params} search={search}>
      <Plus />
    </Link>
  );
};

export const closeLink = (wid: string, loc: string, node_loc: string) => {
  const params = { wid, loc };
  const search = (prev: { openEvals?: string[] | undefined }) => {
    const openEvals = prev.openEvals?.filter((x) => !x.startsWith(node_loc));
    return { openEvals };
  };
  return (
    <Link to="/workflows/$wid/nodes/$loc" params={params} search={search}>
      <Minus style={{ width: 48, height: 48 }} />
    </Link>
  );
};
