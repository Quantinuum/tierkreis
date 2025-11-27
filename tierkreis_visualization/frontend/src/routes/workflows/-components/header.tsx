import { ColumnSort, SortingState } from "@tanstack/react-table";
import { ArrowUp, ArrowDownNarrowWide, ArrowUpNarrowWide } from "lucide-react";
import { Dispatch, SetStateAction } from "react";

export const ColumnHeader = (props: {
  id: string;
  current_sort: ColumnSort;
  title: string | undefined;
  setSorting: Dispatch<SetStateAction<SortingState>>;
}) => {
  const sort = props.current_sort;
  const toggleSorting = (col: string) => {
    if (sort.id == col)
      return props.setSorting([{ id: col, desc: !sort.desc }]);
    return props.setSorting([{ id: col, desc: false }]);
  };

  let Tag = ArrowUp;
  if (sort.id === props.id && sort.desc) Tag = ArrowDownNarrowWide;
  if (sort.id === props.id && !sort.desc) Tag = ArrowUpNarrowWide;

  return (
    <th
      className="text-left cursor-pointer p-4"
      onClick={(_) => toggleSorting(props.id)}
    >
      <div className="flex select-none">
        <Tag className="mr-2" />
        <div>{props.title}</div>
      </div>
    </th>
  );
};
