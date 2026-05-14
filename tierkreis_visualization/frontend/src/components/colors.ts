export const bg_color = (status: string) => {
  switch (status) {
    case "Started":
      return "bg-nexus-purple text-white";
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
      return "border-nexus-purple";
    case "Finished":
      return "border-nexus-green";
    case "Error":
      return "border-nexus-red";
    default:
      return "border-card";
  }
};
