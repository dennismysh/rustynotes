import { createSignal } from "solid-js";

const STORAGE_KEY = "rustynotes:onboarding";

interface OnboardingState {
  welcomed: boolean;
  tips_seen: string[];
}

function load(): OnboardingState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore corrupt data */ }
  return { welcomed: false, tips_seen: [] };
}

function persist(state: OnboardingState): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

const initial = load();

const [isFirstRun, setIsFirstRun] = createSignal(!initial.welcomed);
const [tipsSeen, setTipsSeen] = createSignal<string[]>(initial.tips_seen);

export function markWelcomed(): void {
  setIsFirstRun(false);
  const state = load();
  state.welcomed = true;
  persist(state);
}

export function dismissTip(tipId: string): void {
  setTipsSeen((prev) => [...prev, tipId]);
  const state = load();
  if (!state.tips_seen.includes(tipId)) {
    state.tips_seen.push(tipId);
    persist(state);
  }
}

export function isTipSeen(tipId: string): boolean {
  return tipsSeen().includes(tipId);
}

export function resetOnboarding(): void {
  localStorage.removeItem(STORAGE_KEY);
  setIsFirstRun(true);
  setTipsSeen([]);
}

export { isFirstRun };
