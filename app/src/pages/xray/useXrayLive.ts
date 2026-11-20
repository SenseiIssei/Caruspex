// The live values that hang on their part in the X-ray view.
import { useEffect, useRef, useState } from "react";
import { call } from "../../api";
import type { LiveRow } from "../../api";
import { useInterval } from "../../lib/hooks";
import { live } from "../../state/session";
import { LIVE_KEYS } from "./names";

export function useXrayLive({ active, connected, supported }: { active: boolean; connected: boolean; supported: number[] }) {
  const [liveRows, setLiveRows] = useState<LiveRow[]>([]);
  useInterval(
    () => {
      const rows = LIVE_KEYS.map((k) => live.get(k.pid)).filter((r): r is LiveRow => !!r);
      setLiveRows((old) => (old.length === rows.length && old.every((r, i) => r === rows[i]) ? old : rows));
    },
    active && connected ? 400 : null,
  );
  useEffect(() => {
    if (!connected) setLiveRows([]);
  }, [connected]);
  const [liveOn, setLiveOn] = useState(false);
  const startedLive = useRef(false);
  const toggleLive = () => {
    if (liveOn) {
      void call("stop_live");
      startedLive.current = false;
      setLiveOn(false);
      return;
    }
    const pids = LIVE_KEYS.map((k) => k.pid).filter((p) => supported.length === 0 || supported.includes(p));
    void call("start_live", { pids });
    startedLive.current = true;
    setLiveOn(true);
  };
  useEffect(() => {
    if ((active && connected) || !startedLive.current) return;
    startedLive.current = false;
    setLiveOn(false);
    if (connected) void call("stop_live");
  }, [active, connected]);

  return { liveRows, liveOn, toggleLive };
}
