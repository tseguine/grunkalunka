export function run() {
  const events = [];
  const value = { valueOf() { events.push("valueOf"); return 2; } };
  const result = value + (events.push("rhs"), 3);
  return [result, events.join(","), "2" + 3, 1 / -0, 0 / 0];
}
