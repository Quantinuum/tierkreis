import { Button } from "@/components/ui/button";
import { NodeType } from "@/data/api_types";
import { Link, useParams } from "@tanstack/react-router";
import { Minus, Plus } from "lucide-react";

type SearchData = {
  attempt?: number;
  openEvals?: string[] | undefined;
  openLoops?: string[] | undefined;
  openMaps?: string[] | undefined;
};

export const ZoomInButton = (props: {
  wid: string;
  loc: string;
  node_loc: string;
  node_type: NodeType;
}) => {
  const { wid, loc, node_loc, node_type } = props;
  const { wid: workflowDefinitionId } = useParams({ strict: false });

  const params = { wrid: wid, loc };
  const search = (prev: SearchData): SearchData => {
    const openEvals = [...(prev.openEvals ?? [])];
    const openLoops = [...(prev.openLoops ?? [])];
    const openMaps = [...(prev.openMaps ?? [])];

    if (node_type == "eval") openEvals.push(node_loc);
    else if (node_type == "loop") openLoops.push(node_loc);
    else if (node_type == "map") openMaps.push(node_loc);

    return { attempt: prev.attempt, openEvals, openLoops, openMaps };
  };
  return (
    <Button className="flex-none m-1" variant="default" size="icon">
      {workflowDefinitionId ? (
        <Link
          to="/workflows/$wid/nodes/$loc"
          params={{ wid, loc }}
          search={search}
        >
          <Plus />
        </Link>
      ) : (
        <Link to="/runs/$wrid/nodes/$loc" params={params} search={search}>
          <Plus />
        </Link>
      )}
    </Button>
  );
};

export const ZoomOutButton = (props: {
  wid: string;
  loc: string;
  node_loc: string;
}) => {
  const { wid: workflowDefinitionId } = useParams({ strict: false });
  const params = { wrid: props.wid, loc: props.loc };
  const search = (prev: SearchData): SearchData => {
    const openEvals = prev.openEvals?.filter(
      (x) => !x.startsWith(props.node_loc),
    );
    const openLoops = prev.openLoops?.filter(
      (x) => !x.startsWith(props.node_loc),
    );
    const openMaps = prev.openMaps?.filter(
      (x) => !x.startsWith(props.node_loc),
    );
    return { attempt: prev.attempt, openEvals, openLoops, openMaps };
  };
  return (
    <Button className="flex-none m-1" variant="secondary" size="icon">
      {workflowDefinitionId ? (
        <Link
          to="/workflows/$wid/nodes/$loc"
          params={{ wid: props.wid, loc: props.loc }}
          search={search}
        >
          <Minus />
        </Link>
      ) : (
        <Link to="/runs/$wrid/nodes/$loc" params={params} search={search}>
          <Minus />
        </Link>
      )}
    </Button>
  );
};
