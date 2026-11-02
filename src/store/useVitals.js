import { useEffect, useState } from "react";
import { getVitals, onVitals } from "../lib/ipc";

let state = { vitals: null };
const subscribers = new Set();
let started = false;

function emit() {
  for (const fn of subscribers) fn(state);
}

function applyUpdate(payload) {
  if (!payload) return;
  const { bt, ...rest } = payload;
  state = { vitals: { ...rest, devices: bt ?? [] } };
  emit();
}

async function start() {
  if (started) return;
  started = true;
  try {
    const latest = await getVitals();
    if (latest) applyUpdate(latest);
  } catch {
    /* backend not ready yet */
  }
  try {
    await onVitals((payload) => applyUpdate(payload));
  } catch {
    /* events unavailable (running in a plain browser) */
  }
}

/** Shows a microphone change right away instead of waiting for the next poll. */
export function updateMicMuted(muted) {
  if (!state.vitals) return;
  state = { vitals: { ...state.vitals, mic: { muted } } };
  emit();
}

export function useVitals() {
  const [snapshot, setSnapshot] = useState(state);

  useEffect(() => {
    subscribers.add(setSnapshot);
    start();
    return () => {
      subscribers.delete(setSnapshot);
    };
  }, []);

  return snapshot;
}
