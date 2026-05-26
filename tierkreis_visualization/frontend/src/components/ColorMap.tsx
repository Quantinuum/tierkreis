import { bg_color } from "./colors";

export const ColorMap = (props: { state: string[] }) => {
  return (
    <div className="flex gap-2">
      {props.state.map((state) => (
        <div key={state} className="flex flex-col items-center">
          <div
            className={"w-6 h-6 border-1 rounded shadow-sm " + bg_color(state)}
          />
          <span className="text-xs ">{state}</span>
        </div>
      ))}
    </div>
  );
};
