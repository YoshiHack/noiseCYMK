import { useState } from "react";

export function Settings() {
  const [conflictSafe, setConflictSafe] = useState(false);
  const [httpEnabled, setHttpEnabled] = useState(false);

  return (
    <div className="rounded-xl border border-slate-700 bg-bg-panel p-4">
      <h2 className="mb-3 text-lg font-semibold">Settings</h2>

      <label className="mb-3 flex items-start justify-between gap-3">
        <div>
          <div className="text-sm font-medium">Conflict-safe discovery</div>
          <div className="text-xs text-slate-400">
            Don't fight Govee Home for UDP 4002. Slower discovery but lets the
            official app keep running.
          </div>
        </div>
        <input
          type="checkbox"
          checked={conflictSafe}
          onChange={(e) => setConflictSafe(e.target.checked)}
          className="mt-1 h-4 w-4 accent-accent"
        />
      </label>

      <label className="mb-3 flex items-start justify-between gap-3">
        <div>
          <div className="text-sm font-medium">
            LAN HTTP control <span className="text-slate-500">(off)</span>
          </div>
          <div className="text-xs text-slate-400">
            Expose a tiny REST API on http://&lt;lan-ip&gt;:7878 so your phone can
            control lights. Bearer token required. Off by default.
          </div>
        </div>
        <input
          type="checkbox"
          checked={httpEnabled}
          onChange={(e) => setHttpEnabled(e.target.checked)}
          className="mt-1 h-4 w-4 accent-accent"
        />
      </label>

      <p className="mt-2 text-[11px] text-slate-500">
        Persistence to %APPDATA%/LightSync/settings.json wires up in Phase 4.4.
      </p>
    </div>
  );
}