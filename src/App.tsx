import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, Event } from "@tauri-apps/api/event";
import "./App.css";
import locales from "./locales.json";

interface Config {
  "buffer-size": number;
  "window-size": number;
  "moving-threshold": number;
  "angle-threshold": number;
  "corner-threshold": number;
  "allow-diagonal": boolean;
  debug: boolean;
  language: string;
  gesture: any[];
}

interface LogEntry {
  id: number;
  text: string;
  time: string;
}

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [isDirty, setIsDirty] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const currentLang = config?.language || "en";
  const t = (key: string) => {
    // @ts-ignore
    return locales[currentLang]?.[key] || locales["en"]?.[key] || key;
  };

  useEffect(() => {
    loadConfig();

    const handleContextMenu = (e: MouseEvent) => e.preventDefault();
    window.addEventListener("contextmenu", handleContextMenu);

    const unlistenPromise = listen<string[]>(
      "gesture-detected",
      (event: Event<string[]>) => {
        const newLog: LogEntry = {
          id: Date.now(),
          text: event.payload.join(" → "),
          time: new Date().toLocaleTimeString(),
        };
        setLogs((prev: LogEntry[]) => [newLog, ...prev].slice(0, 50));
      },
    );

    return () => {
      unlistenPromise.then((f: () => void) => f());
      window.removeEventListener("contextmenu", handleContextMenu);
    };
  }, []);

  async function loadConfig() {
    try {
      const cfg: Config = await invoke("get_config");
      setConfig(cfg);
      setIsDirty(false);
    } catch (e) {
      console.error("Failed to load config", e);
    }
  }

  const updateField = (field: keyof Config, value: any) => {
    if (!config) return;
    const newConfig = { ...config, [field]: value };
    setConfig(newConfig);
    setIsDirty(true);
    invoke("update_config", { config: newConfig });
  };

  async function handleSave() {
    try {
      await invoke("save_config");
      setIsDirty(false);
    } catch (e) {
      alert("Failed to save: " + e);
    }
  }

  async function handleReset() {
    if (confirm(t("confirm_reset"))) {
      try {
        const cfg: Config = await invoke("reload_config");
        setConfig(cfg);
        setIsDirty(false);
      } catch (e) {
        alert("Failed to reload: " + e);
      }
    }
  }

  const showCredits = () =>
    alert(
      "Gescher v0.1.0\nDeveloped by Antigravity AI\nA powerful gesture-based system automation tool.",
    );
  const showLicense = () =>
    alert(
      "License: MIT\nCopyright (c) 2026 Antigravity Team\n\nPermission is hereby granted, free of charge, to any person obtaining a copy...",
    );

  async function handleSaveAndExit() {
    try {
      await invoke("save_config");
      await invoke("exit_app");
    } catch (e) {
      alert("Failed to save and exit: " + e);
    }
  }

  if (!config) return <div className="container">Loading...</div>;

  return (
    <div className="container">
      {/* Menu Bar */}
      <div className="reg-menu">
        <div className="reg-menu-top-item">
          {t("menu_registry")}
          <div className="reg-dropdown">
            <div className="reg-dropdown-item" onClick={handleSave}>
              <span>{t("menu_save")}</span>
              <span className="shortcut">Ctrl+S</span>
            </div>
            <div className="reg-dropdown-item" onClick={handleSaveAndExit}>
              <span>{t("menu_save_exit")}</span>
            </div>
            <div
              className="reg-dropdown-item"
              onClick={() => invoke("exit_app")}
            >
              <span>{t("menu_exit_no_save")}</span>
            </div>
            <div className="reg-dropdown-divider"></div>
            <div className="reg-dropdown-item" onClick={handleReset}>
              <span>{t("menu_reset")}</span>
            </div>
          </div>
        </div>

        <div className="reg-menu-top-item">
          {t("menu_view")}
          <div className="reg-dropdown">
            <div className="reg-dropdown-item">
              <span>{t("menu_language")} &raquo;</span>
              <div className="reg-dropdown" style={{ left: "100%", top: 0 }}>
                {Object.keys(locales).map((lang) => (
                  <div
                    key={lang}
                    className={`reg-dropdown-item ${lang === currentLang ? "active" : ""}`}
                    onClick={() => updateField("language", lang)}
                  >
                    <span>
                      {lang === "en"
                        ? "English"
                        : lang === "ja"
                          ? "日本語"
                          : lang}
                    </span>
                    {lang === currentLang && <span>&check;</span>}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="reg-menu-top-item">
          {t("menu_help")}
          <div className="reg-dropdown">
            <div className="reg-dropdown-item" onClick={showCredits}>
              <span>{t("menu_credits")}</span>
            </div>
            <div className="reg-dropdown-item" onClick={showLicense}>
              <span>{t("menu_license")}</span>
            </div>
            <div className="reg-dropdown-divider"></div>
            <div
              className="reg-dropdown-item"
              onClick={() => alert(t("about_title"))}
            >
              <span>{t("menu_about")}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Address Bar */}
      <div className="reg-address-bar">
        <span style={{ color: "var(--reg-accent)" }}>📁</span>
        <div className="reg-address">{t("addr_settings")}</div>
      </div>

      <div className="reg-body">
        <div className="reg-main-pane">
          <table className="reg-table">
            <thead>
              <tr>
                <th style={{ width: "30%" }}>{t("col_name")}</th>
                <th style={{ width: "20%" }}>{t("col_type")}</th>
                <th style={{ width: "50%" }}>{t("col_data")}</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <span className="reg-icon-val">ab</span> (Default)
                </td>
                <td>REG_SZ</td>
                <td>{t("val_default")}</td>
              </tr>
              <tr title={t("tip_buffer")}>
                <td>
                  <span className="reg-icon-val">01</span> buffer_size
                </td>
                <td>REG_DWORD</td>
                <td>
                  <input
                    type="range"
                    min="128"
                    max="2048"
                    className="reg-slider"
                    value={config["buffer-size"]}
                    onChange={(e) =>
                      updateField("buffer-size", parseInt(e.target.value))
                    }
                  />
                  <span style={{ marginLeft: "10px" }}>
                    ({config["buffer-size"]})
                  </span>
                </td>
              </tr>
              <tr title={t("tip_window")}>
                <td>
                  <span className="reg-icon-val">01</span> window_size
                </td>
                <td>REG_DWORD</td>
                <td>
                  <input
                    type="range"
                    min="2"
                    max="50"
                    className="reg-slider"
                    value={config["window-size"]}
                    onChange={(e) =>
                      updateField("window-size", parseInt(e.target.value))
                    }
                  />
                  <span style={{ marginLeft: "10px" }}>
                    ({config["window-size"]})
                  </span>
                </td>
              </tr>
              <tr title={t("tip_moving")}>
                <td>
                  <span className="reg-icon-val">01</span> moving_threshold
                </td>
                <td>REG_FLOAT</td>
                <td>
                  <input
                    type="range"
                    min="1"
                    max="50"
                    step="0.5"
                    className="reg-slider"
                    value={config["moving-threshold"]}
                    onChange={(e) =>
                      updateField(
                        "moving-threshold",
                        parseFloat(e.target.value),
                      )
                    }
                  />
                  <span style={{ marginLeft: "10px" }}>
                    ({config["moving-threshold"].toFixed(1)})
                  </span>
                </td>
              </tr>
              <tr title={t("tip_angle")}>
                <td>
                  <span className="reg-icon-val">01</span> angle_threshold
                </td>
                <td>REG_DWORD</td>
                <td>
                  <input
                    type="range"
                    min="5"
                    max="90"
                    className="reg-slider"
                    value={config["angle-threshold"]}
                    onChange={(e) =>
                      updateField("angle-threshold", parseInt(e.target.value))
                    }
                  />
                  <span style={{ marginLeft: "10px" }}>
                    ({config["angle-threshold"]}°)
                  </span>
                </td>
              </tr>
              <tr title={t("tip_corner")}>
                <td>
                  <span className="reg-icon-val">01</span> corner_threshold
                </td>
                <td>REG_DWORD</td>
                <td>
                  <input
                    type="range"
                    min="5"
                    max="90"
                    className="reg-slider"
                    value={config["corner-threshold"]}
                    onChange={(e) =>
                      updateField("corner-threshold", parseInt(e.target.value))
                    }
                  />
                  <span style={{ marginLeft: "10px" }}>
                    ({config["corner-threshold"]}°)
                  </span>
                </td>
              </tr>
              <tr title={t("tip_diagonal")}>
                <td>
                  <span className="reg-icon-val">ab</span> allow_diagonal
                </td>
                <td>REG_SZ</td>
                <td>
                  <input
                    type="checkbox"
                    checked={config["allow-diagonal"]}
                    onChange={(e) =>
                      updateField("allow-diagonal", e.target.checked)
                    }
                  />
                  <span style={{ marginLeft: "8px" }}>
                    {config["allow-diagonal"] ? "true" : "false"}
                  </span>
                </td>
              </tr>
              <tr title={t("tip_debug")}>
                <td>
                  <span className="reg-icon-val">ab</span> debug_mode
                </td>
                <td>REG_SZ</td>
                <td>
                  <input
                    type="checkbox"
                    checked={config["debug"]}
                    onChange={(e) => updateField("debug", e.target.checked)}
                  />
                  <span style={{ marginLeft: "8px" }}>
                    {config["debug"] ? "enabled" : "disabled"}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div className="reg-footer">
        <div className="reg-actions">
          <button className="reg-btn" onClick={() => invoke("hide_window")}>
            {t("btn_minimize")}
          </button>
          <button className="reg-btn" disabled={!isDirty} onClick={handleSave}>
            {t("menu_registry")} &gt; {t("menu_save")}
          </button>
          <button
            className="reg-btn reg-btn-danger"
            onClick={() => invoke("exit_app")}
          >
            {t("btn_exit")}
          </button>
          <div style={{ flex: 1 }}></div>
          {isDirty && (
            <span
              style={{
                color: "var(--reg-accent)",
                fontSize: "12px",
                alignSelf: "center",
              }}
            >
              {t("status_modified")}
            </span>
          )}
        </div>

        <div className="reg-log-overlay">
          {logs.map((log: LogEntry) => (
            <div key={log.id} className="reg-log-item">
              <span className="time">[{log.time}]</span>
              <span className="msg">{log.text}</span>
            </div>
          ))}
          {logs.length === 0 && (
            <div style={{ color: "#6c7086" }}>{t("log_empty")}</div>
          )}
        </div>

        <div className="reg-status-bar">
          <span>Gescher Registry Editor</span>
          <span>{t("status_items")}</span>
        </div>
      </div>
    </div>
  );
}

export default App;
