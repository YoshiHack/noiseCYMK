import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ProbeReport {
  ip: string;
  arp_visible: boolean;
  scan_multicast_received: boolean;
  unicast_4001_received: boolean;
  unicast_4003_received: boolean;
  raw_responses: string[];
  verdict: string;
}

const PRESETS = [
  { label: "Light bar 1", ip: "192.168.1.192" },
  { label: "Light bar 2", ip: "192.168.1.189" },
  { label: "Strip", ip: "192.168.1.160" },
];

export function Diagnostics() {
  const [ip, setIp] = useState("192.168.1.192");
  const [report, setReport] = useState<ProbeReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (overrideIp?: string) => {
    const target = overrideIp ?? ip;
    setBusy(true);
    setError(null);
    setReport(null);
    try {
      const r = await invoke<ProbeReport>("diagnose_device", { ip: target });
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const row = (label: string, ok: boolean | undefined, help: string) => (
    <div className="flex items-start gap-3 border-b border-slate-800 py-2 last:border-b-0">
      <span
        className={`mt-1 inline-block h-2 w-2 shrink-0 rounded-full ${
          ok === true
            ? "bg-emerald-400"
            : ok === false
            ? "bg-red-400"
            : "bg-slate-500"
        }`}
      />
      <div className="flex-1">
        <div className="text-sm text-slate-100">{label}</div>
        <div className="text-xs text-slate-400">{help}</div>
      </div>
      <span
        className={`text-xs ${
          ok === true
            ? "text-emerald-300"
            : ok === false
            ? "text-red-300"
            : "text-slate-400"
        }`}
      >
        {ok === true ? "OK" : ok === false ? "no reply" : "—"}
      </span>
    </div>
  );

  return (
    <div className="rounded-xl border border-slate-700 bg-bg-panel p-4">
      <h2 className="mb-3 text-lg font-semibold">Diagnose</h2>
      <p className="mb-3 text-xs text-slate-400">
        Probes a single device IP with the same patterns Govee Home and
        SignalRGB use. Use this when "Rescan LAN" finds nothing.
      </p>

      <div className="mb-3 flex flex-wrap gap-2">
        {PRESETS.map((p) => (
          <button
            key={p.ip}
            type="button"
            onClick={() => {
              setIp(p.ip);
              run(p.ip);
            }}
            disabled={busy}
            className="rounded-md border border-slate-700 bg-bg-elev px-2 py-1 text-xs text-slate-200 hover:border-slate-500 disabled:opacity-50"
          >
            {p.label} ({p.ip})
          </button>
        ))}
      </div>

      <div className="mb-3 flex items-center gap-2">
        <input
          type="text"
          value={ip}
          onChange={(e) => setIp(e.target.value)}
          placeholder="192.168.1.x"
          className="flex-1 rounded-md border border-slate-700 bg-bg-elev px-3 py-1.5 font-mono text-sm text-slate-100"
        />
        <button
          type="button"
          onClick={() => run()}
          disabled={busy || !ip.trim()}
          className="rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-slate-900 disabled:opacity-50"
        >
          {busy ? "Probing…" : "Probe"}
        </button>
      </div>

      {error && (
        <div className="mb-3 rounded-lg border border-red-700 bg-red-950/40 p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      {report && (
        <div className="rounded-md border border-slate-700 bg-bg-elev p-3">
          <div className="mb-2 font-mono text-xs text-slate-400">
            Report for {report.ip}
          </div>
          {row(
            "Multicast scan (239.255.255.250:4001)",
            report.scan_multicast_received,
            "Standard Govee discovery channel."
          )}
          {row(
            "Unicast scan (port 4001)",
            report.unicast_4001_received,
            "Fallback when multicast is blocked but LAN Control is on."
          )}
          {row(
            "Unicast status (port 4003)",
            report.unicast_4003_received,
            "Control port — used for color/brightness commands."
          )}

          {report.raw_responses.length > 0 && (
            <details className="mt-2">
              <summary className="cursor-pointer text-xs text-slate-400">
                Raw responses ({report.raw_responses.length})
              </summary>
              <pre className="mt-2 max-h-40 overflow-auto rounded bg-bg-base p-2 font-mono text-[11px] text-slate-300">
                {report.raw_responses.join("\n")}
              </pre>
            </details>
          )}

          <div
            className={`mt-3 whitespace-pre-wrap rounded p-2 text-xs ${
              report.scan_multicast_received ||
              report.unicast_4001_received ||
              report.unicast_4003_received
                ? "bg-emerald-950/40 text-emerald-200"
                : "bg-amber-950/40 text-amber-200"
            }`}
          >
            {report.verdict}
          </div>
        </div>
      )}
    </div>
  );
}