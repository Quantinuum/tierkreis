import { loc_peek, loc_steps } from "@/data/loc";
import { Link } from "@tanstack/react-router";

const Breadcrumb = (props: { wid: string; loc: string }) => {
  const name = loc_peek(props.loc);
  return (
    <>
      <div>/</div>
      <Link
        to={"/workflows/$wid/nodes/$loc"}
        params={props}
        className="m-1 p-2"
      >
        {name}
      </Link>
    </>
  );
};

export const Breadcrumbs = (props: { wid: string; loc: string }) => {
  const steps = loc_steps(props.loc);
  const slices = steps.map((_, i) => steps.slice(0, i + 1).join("."));
  const crumbs = slices.map((x) => <Breadcrumb wid={props.wid} loc={x} />);
  return (
    <div className="border-b-1 h-12 flex items-center">
      <Link to={"/workflows"} className="m-1 p-2">
        workflows
      </Link>
      {crumbs}
    </div>
  );
};
