import { selectVisible } from "grunkalunka:a1/ui";

export function run(expanded: number, count: number): number {
  return selectVisible(expanded > 0 ? 0 : 1, count);
}
