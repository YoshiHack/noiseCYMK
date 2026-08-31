import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface DeviceSummary {
  id: string;
  sku: string;
  ip: string;
  friendly_name: string;
  online: boolean;
  power: Option<boolean>;
  brightness: Option<number>;
  color: Option<[number, number, number]>;
  description: string;
}

export type Option<T> = T | null;

export function useDevices() {
  const [devices, setDevices] = useState<DeviceSummary[]>([]);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<DeviceSummary[]>("list_devices");
      setDevices(list);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const rescan = useCallback(async () => {
    setScanning(true);
    setError(null);
    try {
      await invoke<number>("rescan");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, [refresh]);

  const setColor = useCallback(
    async (id: string, r: number, g: number, b: number) => {
      try {
        await invoke("set_color", { id, r, g, b });
      } catch (e) {
        setError(String(e));
      }
    },
    [],
  );

  const setBrightness = useCallback(async (id: string, pct: number) => {
    try {
      await invoke("set_brightness", { id, pct });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const setPower = useCallback(async (id: string, on: boolean) => {
    try {
      await invoke("set_power", { id, on });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { devices, scanning, error, refresh, rescan, setColor, setBrightness, setPower };
}