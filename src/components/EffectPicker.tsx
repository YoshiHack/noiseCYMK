import { useEffect } from "react";
import { useEffectStore, type Effect } from "../hooks/useEffectStore";
import { invoke } from "@tauri-apps/api/core";

export function EffectPicker() {
  const effect = useEffectStore((s) => s.current);
  const setEffect = useEffectStore((s) => s.set);

  useEffect(() => {
    invoke("start_effect", { args: { effect } }).catch(console.error);
  }, [effect]);

  const apply = (e: Effect) => setEffect(e);

  return (
    <div className="rounded-xl border border-slate-700 bg-bg-panel p-4">
      <h2 className="mb-3 text-lg font-semibold">Effect</h2>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {(
          [
            { kind: "solid", label: "Solid", preview: "#7dd3fc" },
            { kind: "breathing", label: "Breathing", preview: "#a78bfa" },
            { kind: "rainbow", label: "Rainbow", preview: "linear-gradient(90deg,#ef4444,#f59e0b,#10b981,#06b6d4,#6366f1,#ec4899)" },
            { kind: "screen_sync", label: "Screen sync (CMYK)", preview: "linear-gradient(90deg,#06b6d4,#ec4899,#facc15,#0f172a)" },
          ] as const
        ).map((opt) => {
          const active = effect.kind === opt.kind;
          return (
            <button
              key={opt.kind}
              type="button"
              onClick={() => {
                const e: Effect =
                  opt.kind === "solid"
                    ? { kind: "solid", color: [125, 211, 252] }
                    : opt.kind === "breathing"
                    ? { kind: "breathing", color: [167, 139, 250], period_ms: 3000 }
                    : opt.kind === "rainbow"
                    ? { kind: "rainbow", period_ms: 10000 }
                    : { kind: "screen_sync" };
                apply(e);
              }}
              className={`rounded-lg border p-3 text-left transition ${
                active
                  ? "border-accent bg-accent/10"
                  : "border-slate-700 bg-bg-elev hover:border-slate-500"
              }`}
            >
              <div
                className="mb-2 h-8 w-full rounded"
                style={{ background: opt.preview }}
              />
              <div className="text-sm font-medium">{opt.label}</div>
            </button>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-slate-500">
        Screen sync drives each light with a different CMYK channel of the
        on-screen image (Cyan / Magenta / Yellow / Key). Configure which
        device gets which channel by re-ordering your device list.
      </p>
    </div>
  );
}