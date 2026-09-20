const check = process.argv[2] ?? "unknown";
console.error(
  `${check} is intentionally deferred beyond the bootstrap scaffold.`,
);
process.exitCode = 2;
