import { DeviceList } from "./components/DeviceList";
import { EffectPicker } from "./components/EffectPicker";
import { Settings } from "./components/Settings";

export default function App() {
  return (
    <div className="min-h-full bg-bg-base p-6 text-slate-100">
      <header className="mb-6 flex items-baseline justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">LightSync</h1>
          <p className="text-xs text-slate-400">
            Local Govee control — replacement for SignalRGB.
          </p>
        </div>
        <span className="rounded-md border border-slate-700 px-2 py-0.5 text-[10px] uppercase tracking-wider text-slate-400">
          v0.1.0
        </span>
      </header>

      <main className="space-y-6">
        <DeviceList />
        <EffectPicker />
        <Settings />
      </main>

      <footer className="mt-8 text-center text-[11px] text-slate-500">
        Talks directly to your lights over LAN. No cloud, no subscription.
      </footer>
    </div>
  );
}