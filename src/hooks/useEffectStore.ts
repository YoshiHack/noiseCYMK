import { create } from "zustand";

export type Rgb = readonly [number, number, number];

export type Effect =
  | { kind: "solid"; color: Rgb }
  | { kind: "breathing"; color: Rgb; period_ms: number }
  | { kind: "rainbow"; period_ms: number }
  | { kind: "screen_sync" };

interface EffectStore {
  current: Effect;
  set: (e: Effect) => void;
}

export const useEffectStore = create<EffectStore>((set) => ({
  current: { kind: "solid", color: [125, 211, 252] },
  set: (e) => set({ current: e }),
}));