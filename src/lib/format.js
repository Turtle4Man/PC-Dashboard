export function formatMs(ms) {
  if (!Number.isFinite(ms) || ms <= 0) return "0 ms";
  if (ms < 1) return `${ms.toFixed(2)} ms`;
  if (ms < 10) return `${ms.toFixed(1)} ms`;
  return `${Math.round(ms)} ms`;
}

export function latencyLevel(ms) {
  if (ms < 10) return "ok";
  if (ms < 40) return "warn";
  return "bad";
}

export function pingLevel(ms) {
  if (ms < 20) return "ok";
  if (ms < 100) return "warn";
  return "bad";
}

export function percentLevel(percent) {
  if (percent >= 90) return "bad";
  if (percent >= 80) return "warn";
  return "ok";
}
