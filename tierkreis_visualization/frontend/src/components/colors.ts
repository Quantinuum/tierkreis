export const bg_color = (status: string) => {
  switch (status) {
    case "Started":
      return "bg-amber-500";
    case "Finished":
      return "bg-nexus-green text-white";
    case "Error":
      return "bg-nexus-red text-white bg-repeat";
    default:
      return "bg-card";
  }
};

export const border_color = (status: string) => {
  switch (status) {
    case "Started":
      return "border-amber-500";
    case "Finished":
      return "border-nexus-green";
    case "Error":
      return "border-nexus-red";
    default:
      return "border-card";
  }
};
