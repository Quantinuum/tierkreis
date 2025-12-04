export const loc_depth = (loc: string): number => loc.split(".").length;
export const loc_steps = (loc: string): string[] => loc.split(".");
export const loc_parent = (loc: string): string =>
  loc.split(".").slice(0, -1).join(".");
export const loc_peek = (loc: string): string | undefined =>
  loc.split(".").at(-1);
export const loc_children = (loc: string, ns: string[]): string[] =>
  ns.filter((x) => x.startsWith(loc) && loc_depth(x) == loc_depth(loc) + 1);
