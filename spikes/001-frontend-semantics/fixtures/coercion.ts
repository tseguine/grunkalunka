export function run(): unknown[] {
  const events: string[] = [];
  const value = { valueOf() { events.push("valueOf"); return 2; } };
  const result = (value as any) + (events.push("rhs"), 3);
  return [result, events.join(","), "2" + 3, 1 / -0, 0 / 0];
}
