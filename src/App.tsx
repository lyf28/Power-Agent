import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface BatteryStatus {
  percentage: number;
  plugged_in: boolean;
  charging: boolean;
  remaining_seconds: number | null;
}

interface DisplayInfo {
  refresh_rate: number;
}

function formatRemainingTime(seconds: number | null) {
  if (seconds === null) {
    return "Unavailable";
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  return `${hours}h ${minutes}m`;
}

function App() {
  const [battery, setBattery] = useState<BatteryStatus | null>(null);
  const [display, setDisplay] = useState<DisplayInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function loadBatteryStatus() {
    try {
      const result = await invoke<BatteryStatus>("get_battery_status");

      console.log("Battery status:", result);

      setBattery(result);
      setError(null);
    } catch (err) {
      console.error("Failed to get battery status:", err);
      setError(String(err));
    }
  }

  async function loadDisplayInfo() {
  try {
    const result =
      await invoke<DisplayInfo>("get_display_info");

    console.log("Display info:", result);

    setDisplay(result);
  } catch (err) {
    console.error(
      "Failed to get display info:",
      err
    );
  }
}

  useEffect(() => {
    loadBatteryStatus();
    loadDisplayInfo();

    const interval = setInterval(() => {
      loadBatteryStatus();
      loadDisplayInfo();
    }, 5000);

    return () => {
      clearInterval(interval);
    };
  }, []);

  return (
    <main>
      <h1>Power Agent</h1>

      <h2>System Status</h2>

      {error && <p>Error: {error}</p>}

      {!battery && !error && <p>Loading...</p>}

      {battery && (
        <div>
          <p>Battery: {battery.percentage}%</p>

          <p>
            Power Source: {battery.plugged_in ? "AC" : "Battery"}
          </p>

          <p>
            Charging: {battery.charging ? "Yes" : "No"}
          </p>

          <p>
            Remaining Time:{" "}
            {formatRemainingTime(battery.remaining_seconds)}
          </p>
        </div>
      )}

      {display && (
        <p>
          Refresh Rate: {display.refresh_rate} Hz
        </p>
      )}
    </main>
  );
}

export default App;