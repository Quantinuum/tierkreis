import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { colorForDay, formatDayLabel, type HistoryDay } from "./history";

/** A single day-cell in a run history calendar, with a reliable hover tooltip. */
export const HistoryDot = (props: { day: HistoryDay; sizeClass: string }) => (
  <Tooltip>
    <TooltipTrigger asChild>
      <div
        className={`${props.sizeClass} rounded-full ${colorForDay(props.day)}`}
      />
    </TooltipTrigger>
    <TooltipContent>
      {formatDayLabel(props.day.key)}: {props.day.count} run
      {props.day.count === 1 ? "" : "s"}
      {props.day.erroredLast ? " (latest failed)" : ""}
    </TooltipContent>
  </Tooltip>
);
