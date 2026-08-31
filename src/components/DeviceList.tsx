import { useState } from "react";
import { useDevices } from "../hooks/useDevices";

interface DeviceCardProps {
  id: string;
  sku: string;
  ip: string;
  description: string;
  online: boolean;
  onColor: (id: string, r: number, g: number, b: number) => Promise<void>;
  onBrightness: (id: string, pct: number) => Promise<void>;
  onPower: (id: string, on: boolean) => Promise<void>;
}

export function DeviceCard(props: DeviceCardProps) {
  const [color, setColor] = useState("#7dd3fc");
  const [brightness, setBrightness] = useState(80);
  const [power, setPower] = useState(true);

  const onColorChange = async (hex: string) => {
    setColor(hex);
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    await props.onColor(props.id, r, g, b);
  };

  return (
    <div className="rounded-xl border border-slate-700 bg-bg-panel p-4 shadow-lg">
      <div className="mb-2 flex items-baseline justify-between">
        <h3 className="font-semibold text-slate-100">{props.sku}</h3>
        <span
          className={`text-xs ${props.online ? "text-emerald-400" : "text-slate-500"}`}
        >
          {props.online ? "online" : "offline"}
        </span>
      </div>
      <p className="mb-3 text-xs text-slate-400">{props.description}</p>
      <p className="mb-3 font-mono text-[11px] text-slate-500">{props.ip}</p>

      <div className="mb-3 flex items-center gap-2">
        <label className="text-xs text-slate-400">Color</label>
        <input
          type="color"
          value={color}
          onChange={(e) => onColorChange(e.target.value)}
          className="h-8 w-12 cursor-pointer rounded bg-bg-elev"
        />
        <span className="font-mono text-[10px] text-slate-500">{color}</span>
      </div>

      <div className="mb-3">
        <label className="mb-1 block text-xs text-slate-400">
          Brightness: {brightness}%
        </label>
        <input
          type="range"
          min={0}
          max={100}
          value={brightness}
          onChange={(e) => {
            const v = Number(e.target.value);
            setBrightness(v);
            props.onBrightness(props.id, v);
          }}
          className="w-full accent-accent"
        />
      </div>

      <div className="flex items-center justify-between">
        <span className="text-xs text-slate-400">Power</span>
        <button
          type="button"
          onClick={() => {
            const next = !power;
            setPower(next);
            props.onPower(props.id, next);
          }}
          className={`relative h-6 w-11 rounded-full transition ${
            power ? "bg-accent" : "bg-slate-600"
          }`}
        >
          <span
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition ${
              power ? "left-5" : "left-0.5"
            }`}
          />
        </button>
      </div>
    </div>
  );
}

export function DeviceList() {
  const { devices, scanning, error, rescan, setColor, setBrightness, setPower } =
    useDevices();

  return (
    <div>
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Devices</h2>
        <button
          type="button"
          onClick={rescan}
          disabled={scanning}
          className="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-slate-900 disabled:opacity-50"
        >
          {scanning ? "Scanning…" : "Rescan LAN"}
        </button>
      </div>

      {error && (
        <div className="mb-4 rounded-lg border border-red-700 bg-red-950/40 p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      {devices.length === 0 ? (
        <div className="rounded-xl border border-dashed border-slate-700 bg-bg-panel p-8 text-center text-sm text-slate-400">
          No devices found. Make sure your Govee devices are powered on, on the
          same Wi-Fi as this machine, and have LAN Control enabled in the Govee
          Home app (Device Settings → LAN Control).
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {devices.map((d) => (
            <DeviceCard
              key={d.id}
              id={d.id}
              sku={d.sku}
              ip={d.ip}
              description={d.description}
              online={d.online}
              onColor={setColor}
              onBrightness={setBrightness}
              onPower={setPower}
            />
          ))}
        </div>
      )}
    </div>
  );
}