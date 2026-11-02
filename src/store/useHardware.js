import { useEffect, useState } from "react";
import { getHardware } from "../lib/ipc";

const REFRESH_MS = 3000;

let state = { hardware: null, error: null };
const subscribers = new Set();
let timer = null;
let inflight = false;

function emit() {
  for (const fn of subscribers) fn(state);
}

async function refresh() {
  if (inflight) return;
  inflight = true;
  try {
    const hardware = await getHardware();
    state = { hardware, error: null };
  } catch (reason) {
    state = { hardware: state.hardware, error: String(reason) };
  } finally {
    inflight = false;
    emit();
  }
}

function start() {
  if (timer !== null) return;
  refresh();
  timer = setInterval(refresh, REFRESH_MS);
}

function stop() {
  if (timer === null) return;
  clearInterval(timer);
  timer = null;
}

export function useHardware() {
  const [snapshot, setSnapshot] = useState(state);

  useEffect(() => {
    subscribers.add(setSnapshot);
    start();
    return () => {
      subscribers.delete(setSnapshot);
      if (subscribers.size === 0) stop();
    };
  }, []);

  return snapshot;
}
