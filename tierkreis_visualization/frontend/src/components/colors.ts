export const bg_color = (status: string) => {
  switch (status) {
    case "Started":
      return "bg-chart-4";
    case "Finished":
      return "bg-chart-2";
    case "Error":
      return "bg-chart-1 bg-repeat";
    default:
      return "bg-card";
  }
};

export const border_color = (status: string) => {
  switch (status) {
    case "Started":
      return "border-chart-4";
    case "Finished":
      return "border-chart-2";
    case "Error":
      return "border-chart-1";
    default:
      return "border-card";
  }
};
