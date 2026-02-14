import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, Event } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { FaCheck } from "react-icons/fa";
import "./App.css";

interface Gesture {
  trigger: string;
  command?: string;
  text?: string;
  keys?: string;
  target_bin?: string;
  control?: string;
  "repeat-count"?: number;
  "repeat-interval"?: number;
  _type?: "command" | "text" | "keys";
  enabled?: boolean;
}

interface Config {
  "buffer-size": number;
  "window-size": number;
  "moving-threshold": number;
  "angle-threshold": number;
  "corner-threshold": number;
  "allow-diagonal": boolean;
  debug: boolean;
  language: string;
  gestures?: Gesture[];
}

interface LogEntry {
  id: number;
  text: string;
  time: string;
  app?: string;
}

type ViewMode = "general" | "gestures";

const MODIFIER_ORDER = ["Ctrl", "Shift", "Alt", "Meta"];

const sortKeys = (keys: string[]) => {
  return keys.sort((a, b) => {
    const indexA = MODIFIER_ORDER.indexOf(a);
    const indexB = MODIFIER_ORDER.indexOf(b);

    if (indexA !== -1 && indexB !== -1) {
      return indexA - indexB;
    }
    if (indexA !== -1) return -1;
    if (indexB !== -1) return 1;

    return a.localeCompare(b);
  });
};

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [isDirty, setIsDirty] = useState(false);
  const [lastSaveTime, setLastSaveTime] = useState<number>(0);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [currentView, setCurrentView] = useState<ViewMode>("general");
  const [activeGestureIndex, setActiveGestureIndex] = useState<number | null>(
    null,
  );
  const [recordingGesture, setRecordingGesture] = useState<number | null>(null);
  const [recordingKeys, setRecordingKeys] = useState<number | null>(null);
  const [pressedKeys, setPressedKeys] = useState<Set<string>>(new Set());
  const [capturedKeys, setCapturedKeys] = useState<string>("");
  const [runningProcesses, setRunningProcesses] = useState<string[]>([]);
  const [locales, setLocales] = useState<any>({});

  // useRef to hold the latest config for event listeners
  const configRef = useRef<Config | null>(null);
  const recordingGestureRef = useRef<number | null>(null);
  const maxPressedKeysRef = useRef<Set<string>>(new Set());
  const activeRecordingIndexRef = useRef<number | null>(null);

  // Update configRef whenever config changes
  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    recordingGestureRef.current = recordingGesture;
  }, [recordingGesture]);

  const currentLang = config?.language || "en";
  const t = (key: string) => {
    // @ts-ignore
    return locales[currentLang]?.[key] || locales["en"]?.[key] || key;
  };

  // 初回のみ実行: 設定読み込みとイベントリスナー登録
  useEffect(() => {
    loadLocales();
    loadConfig();
    loadRunningProcesses();

    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
      // 右クリックで記録をキャンセル
      if (recordingGestureRef.current !== null) {
        setRecordingGesture(null);
        invoke("stop_gesture_recording").catch(console.error);
        // activeRecordingIndexRef はクリアしない（ジェスチャ完了イベントを待つ）
        // もし本当にキャンセルされたならイベントは来ない
      }
    };
    window.addEventListener("contextmenu", handleContextMenu);

    interface GestureDetectedPayload {
      gestures: string[];
      app_name: string;
    }

    const unlistenGesturePromise = listen<GestureDetectedPayload>(
      "gesture-detected",
      (event: Event<GestureDetectedPayload>) => {
        const payload = event.payload;
        const newLog: LogEntry = {
          id: Date.now(),
          text: payload.gestures.join(" → "),
          time: new Date().toLocaleTimeString(),
          app: payload.app_name,
        };
        setLogs((prev: LogEntry[]) => [newLog, ...prev].slice(0, 50));
      },
    );

    const unlistenRecordedPromise = listen<string>(
      "gesture-recorded",
      (event: Event<string>) => {
        const index = activeRecordingIndexRef.current;
        if (index !== null) {
          handleGestureChange(index, "trigger", event.payload);
          invoke("stop_gesture_recording").catch(console.error);
          setRecordingGesture(null);
          activeRecordingIndexRef.current = null;
        }
      },
    );

    return () => {
      unlistenGesturePromise.then((f: () => void) => f());
      unlistenRecordedPromise.then((f: () => void) => f());
      window.removeEventListener("contextmenu", handleContextMenu);
    };
  }, []);

  // キー入力記録用のイベントリスナー（recordingKeys が変わったときのみ）
  useEffect(() => {
    if (recordingKeys === null) {
      setPressedKeys(new Set());
      setCapturedKeys("");
      maxPressedKeysRef.current.clear();
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();

      // Escape でキャンセル
      if (e.key === "Escape") {
        setRecordingKeys(null);
        setPressedKeys(new Set());
        setCapturedKeys("");
        maxPressedKeysRef.current.clear();
        return;
      }

      // Enter で確定
      if (e.key === "Enter") {
        let keysToSave = capturedKeys;

        // capturedKeys が空で、現在キーが押されている場合は即座にキャプチャ
        if (!keysToSave && maxPressedKeysRef.current.size > 0) {
          const keysArray = sortKeys(Array.from(maxPressedKeysRef.current));
          keysToSave = keysArray.join("+");
        }

        if (keysToSave) {
          handleGestureChange(recordingKeys, "keys", keysToSave);
        }

        setRecordingKeys(null);
        setPressedKeys(new Set());
        setCapturedKeys("");
        maxPressedKeysRef.current.clear();
        return;
      }

      // キーを追加
      const newKeys = new Set(pressedKeys);

      // 修飾キー
      if (e.ctrlKey && !newKeys.has("Ctrl")) newKeys.add("Ctrl");
      if (e.shiftKey && !newKeys.has("Shift")) newKeys.add("Shift");
      if (e.altKey && !newKeys.has("Alt")) newKeys.add("Alt");
      if (e.metaKey && !newKeys.has("Meta")) newKeys.add("Meta");

      // 通常のキー
      const key = e.key;
      if (
        !["Control", "Shift", "Alt", "Meta", "Enter", "Escape"].includes(key)
      ) {
        const normalizedKey = key.length === 1 ? key.toUpperCase() : key;
        newKeys.add(normalizedKey);
      }

      // maxPressedKeys にも追加
      newKeys.forEach((k) => maxPressedKeysRef.current.add(k));

      setPressedKeys(newKeys);
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();

      const newKeys = new Set(pressedKeys);

      // 修飾キーの解放を検知
      if (e.key === "Control") newKeys.delete("Ctrl");
      if (e.key === "Shift") newKeys.delete("Shift");
      if (e.key === "Alt") newKeys.delete("Alt");
      if (e.key === "Meta") newKeys.delete("Meta");

      // 通常のキーの解放
      const key = e.key;
      if (!["Control", "Shift", "Alt", "Meta"].includes(key)) {
        const normalizedKey = key.length === 1 ? key.toUpperCase() : key;
        newKeys.delete(normalizedKey);
      }

      // すべてのキーが離されたら、maxPressedKeys をキャプチャ
      if (newKeys.size === 0 && maxPressedKeysRef.current.size > 0) {
        const keysArray = sortKeys(Array.from(maxPressedKeysRef.current));
        setCapturedKeys(keysArray.join("+"));
        maxPressedKeysRef.current.clear();
      }

      setPressedKeys(newKeys);
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, [recordingKeys, pressedKeys, capturedKeys]);

  // 設定ファイルとUIの同期（2秒ごとにチェック）
  useEffect(() => {
    const interval = setInterval(async () => {
      // 編集中、または最後の保存から5秒以内の場合はスキップ
      const timeSinceLastSave = Date.now() - lastSaveTime;
      if (isDirty || timeSinceLastSave < 5000) {
        return;
      }

      try {
        const cfg: any = await invoke("get_config");

        // バックエンドから gesture (単数形) で返ってくるので、gestures (複数形) にマッピング
        if (cfg.gesture) {
          cfg.gestures = cfg.gesture;
          delete cfg.gesture;
        }

        if (cfg.gestures) {
          cfg.gestures = cfg.gestures.map((g: any) => ({
            ...g,
            _type:
              g.text !== undefined && g.text !== null
                ? "text"
                : g.keys !== undefined && g.keys !== null
                  ? "keys"
                  : "command",
          }));
        }
        setConfig(cfg);
      } catch (e) {
        console.error("Failed to sync config", e);
      }
    }, 2000); // 2秒ごとにチェック

    return () => clearInterval(interval);
  }, [isDirty, lastSaveTime]);

  async function loadLocales() {
    try {
      const localesJson: string = await invoke("get_locales");
      const localesData = JSON.parse(localesJson);
      setLocales(localesData);
    } catch (e) {
      console.error("Failed to load locales", e);
      // フォールバック: 空のlocalesオブジェクト
      setLocales({ en: {}, ja: {} });
    }
  }

  async function loadConfig() {
    try {
      const cfg: any = await invoke("get_config");

      // バックエンドから gesture (単数形) で返ってくるので、gestures (複数形) にマッピング
      if (cfg.gesture) {
        cfg.gestures = cfg.gesture;
        delete cfg.gesture;
      }

      if (cfg.gestures) {
        cfg.gestures = cfg.gestures.map((g: any) => ({
          ...g,
          _type:
            g.text !== undefined && g.text !== null
              ? "text"
              : g.keys !== undefined && g.keys !== null
                ? "keys"
                : "command",
        }));
      }
      setConfig(cfg);
      setIsDirty(false);
    } catch (e) {
      console.error("Failed to load config", e);
    }
  }

  async function loadRunningProcesses() {
    try {
      const processes: string[] = await invoke("get_running_processes");
      setRunningProcesses(processes);
    } catch (e) {
      console.error("Failed to load running processes", e);
    }
  }

  const updateField = (field: keyof Config, value: any) => {
    if (!config) return;
    const newConfig = { ...config, [field]: value };
    setConfig(newConfig);
    setIsDirty(true);
    invoke("update_config", { config: cleanConfigForBackend(newConfig) })
      .then(() => {
        setIsDirty(false);
        setLastSaveTime(Date.now());
      })
      .catch((e) => {
        console.error("Failed to update config", e);
      });
  };

  // バックエンドに送信する前に _type フィールドを除外
  const cleanGestureForBackend = (gesture: Gesture) => {
    const { _type, ...rest } = gesture;
    return rest;
  };

  const cleanConfigForBackend = (config: Config) => {
    // gesture フィールドを除外し、gestures フィールドのみを含める
    const { gesture, ...rest } = config as any;
    return {
      ...rest,
      gestures: config.gestures?.map(cleanGestureForBackend) || [],
    };
  };

  // ジェスチャの軌跡をアイコンとして描画
  const GestureIcon = ({ trigger }: { trigger: string }) => {
    if (!trigger) return null;
    const parts = trigger.split(" -> ");
    let x = 0;
    let y = 0;
    const points = [{ x, y }];
    let minX = 0,
      maxX = 0,
      minY = 0,
      maxY = 0;

    parts.forEach((dir) => {
      if (dir.includes("UP")) y -= 1;
      if (dir.includes("DOWN")) y += 1;
      if (dir.includes("LEFT")) x -= 1;
      if (dir.includes("RIGHT")) x += 1;
      points.push({ x, y });
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);
    });

    const padding = 0.5;
    const vbX = minX - padding;
    const vbY = minY - padding;
    const vbW = maxX - minX + padding * 2;
    const vbH = maxY - minY + padding * 2;

    const pathData = points
      .map((p, i) => `${i === 0 ? "M" : "L"} ${p.x} ${p.y}`)
      .join(" ");

    return (
      <svg
        width="24"
        height="24"
        viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`}
        style={{
          display: "inline-block",
          verticalAlign: "middle",
          background: "rgba(255,255,255,0.05)",
          borderRadius: "4px",
          padding: "2px",
        }}
      >
        <path
          d={pathData}
          stroke="#cdd6f4"
          strokeWidth="0.2"
          fill="none"
          strokeLinecap="round"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
        {/* Start dot (Green) */}
        <circle cx="0" cy="0" r="0.2" fill="#a6e3a1" />
        {/* End dot (Red) */}
        <circle cx={x} cy={y} r="0.2" fill="#f38ba8" />
      </svg>
    );
  };

  const handleGestureChange = (
    index: number,
    field: keyof Gesture | "_type",
    value: string | number | boolean,
  ) => {
    const config = configRef.current;
    if (!config || !config.gestures) return;

    const newGestures = [...config.gestures];
    const gesture = { ...newGestures[index] };

    if (field === "_type") {
      gesture._type = value as "command" | "text" | "keys";
      if (gesture._type === "command") {
        gesture.text = undefined;
        gesture.keys = undefined;
        gesture["repeat-count"] = undefined;
        gesture["repeat-interval"] = undefined;
        if (!gesture.command) gesture.command = "";
      } else if (gesture._type === "text") {
        gesture.command = undefined;
        gesture.keys = undefined;
        gesture["repeat-count"] = undefined;
        gesture["repeat-interval"] = undefined;
        if (!gesture.text) gesture.text = "";
      } else if (gesture._type === "keys") {
        gesture.command = undefined;
        gesture.text = undefined;
        if (!gesture.keys) gesture.keys = "";
        // Keys タイプの場合、repeat フィールドを保持
      }
    } else if (field === "repeat-count" || field === "repeat-interval") {
      // @ts-ignore
      gesture[field] =
        typeof value === "number"
          ? value
          : parseInt(value as string) || undefined;
    } else {
      // @ts-ignore
      gesture[field] = value;
    }

    newGestures[index] = gesture;
    const newConfig = { ...config, gestures: newGestures };
    setConfig(newConfig);
    setIsDirty(true);
    // 即座にバックエンドに反映
    invoke("update_config", { config: cleanConfigForBackend(newConfig) })
      .then(() => {
        setIsDirty(false);
        setLastSaveTime(Date.now());
      })
      .catch((e) => {
        console.error("Failed to update config", e);
        alert("Failed to save configuration: " + e);
      });
  };

  const addGesture = () => {
    if (!config) return;

    // 選択中のジェスチャの target_bin を取得
    const selectedTargetBin =
      activeGestureIndex !== null && config.gestures?.[activeGestureIndex]
        ? config.gestures[activeGestureIndex].target_bin || ""
        : "";

    const newGesture: Gesture = {
      trigger: "UP -> DOWN",
      _type: "command",
      command: "calc.exe",
      target_bin: selectedTargetBin,
      enabled: true,
    };

    const newConfig = {
      ...config,
      gestures: [...(config.gestures || []), newGesture],
    };

    setConfig(newConfig);
    setIsDirty(true);
    invoke("update_config", { config: cleanConfigForBackend(newConfig) })
      .then(() => {
        setIsDirty(false);
        setLastSaveTime(Date.now());
      })
      .catch((e) => {
        console.error("Failed to add gesture", e);
      });
    setActiveGestureIndex((newConfig.gestures?.length || 1) - 1);
  };

  const removeGesture = (index: number) => {
    if (!config || !config.gestures) return;
    const newGestures = config.gestures.filter((_, i) => i !== index);
    const newConfig = { ...config, gestures: newGestures };
    setConfig(newConfig);
    setIsDirty(true);
    invoke("update_config", { config: cleanConfigForBackend(newConfig) })
      .then(() => {
        setIsDirty(false);
        setLastSaveTime(Date.now());
      })
      .catch((e) => {
        console.error("Failed to remove gesture", e);
      });
  };

  const startGestureRecording = async (index: number) => {
    try {
      await invoke("start_gesture_recording");
      setRecordingGesture(index);
      activeRecordingIndexRef.current = index;
    } catch (e) {
      console.error("Failed to start gesture recording", e);
    }
  };

  const startKeyRecording = (index: number) => {
    if (recordingKeys === index) {
      let keysToSave = capturedKeys;
      if (!keysToSave && maxPressedKeysRef.current.size > 0) {
        const keysArray = sortKeys(Array.from(maxPressedKeysRef.current));
        keysToSave = keysArray.join("+");
      }

      if (keysToSave) {
        handleGestureChange(index, "keys", keysToSave);
      }

      setRecordingKeys(null);
      setPressedKeys(new Set());
      setCapturedKeys("");
      maxPressedKeysRef.current.clear();
    } else {
      setRecordingKeys(index);
    }
  };

  const selectAppFromFile = async (index: number) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Executable",
            extensions: ["exe", "app", "sh"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        const fileName = selected.split(/[\\/]/).pop() || selected;
        handleGestureChange(index, "target_bin", fileName);
      } else {
        // キャンセルされた場合は Global に戻す
        handleGestureChange(index, "target_bin", "");
      }
    } catch (e) {
      console.error("Failed to select file", e);
      // エラーの場合も Global に戻す
      handleGestureChange(index, "target_bin", "");
    }
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
        if (cfg.gestures) {
          cfg.gestures = cfg.gestures.map((g) => ({
            ...g,
            _type: g.text
              ? "text"
              : g.keys
                ? "keys"
                : g.command
                  ? "command"
                  : "command",
          }));
        }
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
  const showLicense = () => alert("License: MIT\nCopyright (c) 2026 cet-t");

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
                    {lang === currentLang && (
                      <span style={{ display: "flex", alignItems: "center" }}>
                        <FaCheck size={12} />
                      </span>
                    )}
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
        <div className="reg-address">
          {t("addr_settings")}
          {currentView === "gestures" ? "\\Gestures" : "\\General"}
        </div>
      </div>

      <div className="reg-body">
        {/* Sidebar */}
        <div className="reg-sidebar">
          <div className="tree-item">
            <span className="tree-icon">💻</span>
            <span>Computer</span>
          </div>
          <div className="tree-item" style={{ paddingLeft: "35px" }}>
            <span className="tree-icon">📂</span>
            <span>HKEY_CURRENT_CONFIG</span>
          </div>
          <div
            className={`tree-item ${currentView === "general" ? "selected" : ""}`}
            style={{ paddingLeft: "50px" }}
            onClick={() => setCurrentView("general")}
          >
            <span className="tree-icon">⚙️</span>
            <span>General</span>
          </div>
          <div
            className={`tree-item ${currentView === "gestures" ? "selected" : ""}`}
            style={{ paddingLeft: "50px" }}
            onClick={() => setCurrentView("gestures")}
          >
            <span className="tree-icon">⚡</span>
            <span>Gestures</span>
          </div>
        </div>

        {/* Main Content */}
        <div className="reg-main-pane">
          {currentView === "general" && (
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
                    <input
                      type="number"
                      min="128"
                      max="2048"
                      className="reg-spinbox"
                      value={config["buffer-size"]}
                      onChange={(e) =>
                        updateField(
                          "buffer-size",
                          parseInt(e.target.value) || 128,
                        )
                      }
                    />
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
                    <input
                      type="number"
                      min="2"
                      max="50"
                      className="reg-spinbox"
                      value={config["window-size"]}
                      onChange={(e) =>
                        updateField(
                          "window-size",
                          parseInt(e.target.value) || 2,
                        )
                      }
                    />
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
                    <input
                      type="number"
                      min="1"
                      max="50"
                      step="0.5"
                      className="reg-spinbox"
                      value={config["moving-threshold"]}
                      onChange={(e) =>
                        updateField(
                          "moving-threshold",
                          parseFloat(e.target.value) || 1,
                        )
                      }
                    />
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
                    <input
                      type="number"
                      min="5"
                      max="90"
                      className="reg-spinbox"
                      value={config["angle-threshold"]}
                      onChange={(e) =>
                        updateField(
                          "angle-threshold",
                          parseInt(e.target.value) || 5,
                        )
                      }
                    />
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
                        updateField(
                          "corner-threshold",
                          parseInt(e.target.value),
                        )
                      }
                    />
                    <input
                      type="number"
                      min="5"
                      max="90"
                      className="reg-spinbox"
                      value={config["corner-threshold"]}
                      onChange={(e) =>
                        updateField(
                          "corner-threshold",
                          parseInt(e.target.value) || 5,
                        )
                      }
                    />
                  </td>
                </tr>
                <tr title={t("tip_diagonal")}>
                  <td>
                    <span className="reg-icon-val">ab</span> allow_diagonal
                  </td>
                  <td>REG_BOOL</td>
                  <td>
                    <input
                      type="checkbox"
                      checked={config["allow-diagonal"]}
                      onChange={(e) =>
                        updateField("allow-diagonal", e.target.checked)
                      }
                    />
                    <span style={{ marginLeft: "8px" }}>
                      {config["allow-diagonal"] ? "TRUE" : "FALSE"}
                    </span>
                  </td>
                </tr>
                <tr title={t("tip_debug")}>
                  <td>
                    <span className="reg-icon-val">ab</span> debug_mode
                  </td>
                  <td>REG_BOOL</td>
                  <td>
                    <input
                      type="checkbox"
                      checked={config["debug"]}
                      onChange={(e) => updateField("debug", e.target.checked)}
                    />
                    <span style={{ marginLeft: "8px" }}>
                      {config["debug"] ? "TRUE" : "FALSE"}
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          )}

          {currentView === "gestures" && (
            <div style={{ display: "flex", flexDirection: "column", flex: 1 }}>
              <table className="reg-table">
                <thead>
                  <tr>
                    <th style={{ width: "5%" }}>{t("col_on")}</th>
                    <th style={{ width: "18%" }}>{t("col_trigger")}</th>
                    <th style={{ width: "12%" }}>{t("col_type")}</th>
                    <th style={{ width: "30%" }}>{t("col_value")}</th>
                    <th style={{ width: "25%" }}>{t("col_target_app")}</th>
                    <th style={{ width: "15%" }}>{t("col_action")}</th>
                  </tr>
                </thead>
                <tbody>
                  {config.gestures?.map((g, idx) => (
                    <tr
                      key={idx}
                      className={activeGestureIndex === idx ? "selected" : ""}
                      onClick={() => setActiveGestureIndex(idx)}
                    >
                      <td style={{ textAlign: "center" }}>
                        <input
                          type="checkbox"
                          checked={g.enabled !== false}
                          onChange={(e) =>
                            handleGestureChange(
                              idx,
                              "enabled",
                              e.target.checked,
                            )
                          }
                          onClick={(e) => e.stopPropagation()}
                        />
                      </td>
                      <td>
                        <div
                          style={{
                            display: "flex",
                            gap: "4px",
                            alignItems: "center",
                          }}
                        >
                          <div
                            className="reg-input-inline"
                            style={{
                              flex: 1,
                              padding: "4px 8px",
                              minHeight: "24px",
                              display: "flex",
                              alignItems: "center",
                            }}
                          >
                            {g.trigger ? (
                              <GestureIcon trigger={g.trigger} />
                            ) : (
                              <span style={{ color: "#6c7086" }}>
                                Record gesture →
                              </span>
                            )}
                          </div>
                          <button
                            className="reg-btn-small"
                            onClick={(e) => {
                              e.stopPropagation();
                              startGestureRecording(idx);
                            }}
                            title="Record gesture"
                            style={{
                              background:
                                recordingGesture === idx
                                  ? "#f38ba8"
                                  : "#45475a",
                              padding: "2px 6px",
                              fontSize: "10px",
                              border: "1px solid #313244",
                              color: "#cdd6f4",
                              cursor: "pointer",
                            }}
                          >
                            {recordingGesture === idx ? "●" : "○"}
                          </button>
                        </div>
                      </td>
                      <td>
                        <select
                          className="reg-input-inline"
                          style={{
                            background: "#1e1e2e",
                            color: "#cdd6f4",
                            border: "1px solid #313244",
                            width: "100%",
                          }}
                          value={g._type || "command"}
                          onChange={(e) =>
                            handleGestureChange(idx, "_type", e.target.value)
                          }
                        >
                          <option value="command">Command</option>
                          <option value="text">Text</option>
                          <option value="keys">Keys</option>
                        </select>
                      </td>
                      <td>
                        <div
                          style={{
                            display: "flex",
                            gap: "4px",
                            alignItems: "center",
                          }}
                        >
                          <input
                            className="reg-input-inline"
                            style={{ flex: 1 }}
                            value={
                              recordingKeys === idx && g._type === "keys"
                                ? capturedKeys ||
                                  sortKeys(
                                    Array.from(maxPressedKeysRef.current),
                                  ).join("+")
                                : g._type === "text"
                                  ? g.text || ""
                                  : g._type === "keys"
                                    ? g.keys || ""
                                    : g.command || ""
                            }
                            onChange={(e) =>
                              handleGestureChange(
                                idx,
                                // @ts-ignore
                                g._type || "command",
                                e.target.value,
                              )
                            }
                            placeholder={
                              recordingKeys === idx && g._type === "keys"
                                ? "Release keys, then press Enter to confirm, Esc to cancel"
                                : g._type === "text"
                                  ? "Text to type..."
                                  : g._type === "keys"
                                    ? "Ctrl+C"
                                    : "calc.exe"
                            }
                            readOnly={
                              g._type === "keys" && recordingKeys === idx
                            }
                          />
                          {g._type === "keys" && (
                            <button
                              className="reg-btn-small"
                              onClick={(e) => {
                                e.stopPropagation();
                                startKeyRecording(idx);
                              }}
                              title="Record key combo"
                              style={{
                                background:
                                  recordingKeys === idx ? "#f38ba8" : "#45475a",
                                padding: "2px 6px",
                                fontSize: "10px",
                                border: "1px solid #313244",
                                color: "#cdd6f4",
                                cursor: "pointer",
                              }}
                            >
                              {recordingKeys === idx ? "●" : "⌨"}
                            </button>
                          )}
                        </div>
                      </td>
                      <td>
                        <div
                          style={{
                            display: "flex",
                            gap: "4px",
                            alignItems: "center",
                          }}
                        >
                          <select
                            className="reg-input-inline"
                            style={{
                              background: "#1e1e2e",
                              color: "#cdd6f4",
                              border: "1px solid #313244",
                              flex: 1,
                            }}
                            value={g.target_bin || ""}
                            onChange={(e) => {
                              handleGestureChange(
                                idx,
                                "target_bin",
                                e.target.value,
                              );
                            }}
                          >
                            <option value="">Global (Any)</option>
                            {g.target_bin &&
                              !runningProcesses.includes(g.target_bin) && (
                                <option value={g.target_bin}>
                                  {g.target_bin}
                                </option>
                              )}
                            <option disabled>──────────</option>
                            {runningProcesses.map((proc) => (
                              <option key={proc} value={proc}>
                                {proc}
                              </option>
                            ))}
                          </select>
                          <button
                            className="reg-btn-small"
                            onClick={(e) => {
                              e.stopPropagation();
                              selectAppFromFile(idx);
                            }}
                            title="Browse for executable"
                            style={{
                              background: "#45475a",
                              padding: "2px 6px",
                              fontSize: "10px",
                              border: "1px solid #313244",
                              color: "#cdd6f4",
                              cursor: "pointer",
                            }}
                          >
                            📁
                          </button>
                        </div>
                      </td>
                      <td style={{ textAlign: "center" }}>
                        <button
                          className="reg-btn reg-btn-danger"
                          style={{ padding: "2px 8px", fontSize: "10px" }}
                          onClick={(e) => {
                            e.stopPropagation();
                            removeGesture(idx);
                          }}
                        >
                          X
                        </button>
                      </td>
                    </tr>
                  ))}
                  {(!config.gestures || config.gestures.length === 0) && (
                    <tr>
                      <td
                        colSpan={5}
                        style={{ textAlign: "center", fontStyle: "italic" }}
                      >
                        No gestures configured.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
              <div style={{ padding: "10px", background: "#1e1e2e" }}>
                <button className="reg-btn" onClick={addGesture}>
                  + Add New Gesture
                </button>
                {recordingGesture !== null && (
                  <span style={{ marginLeft: "15px", color: "#f38ba8" }}>
                    🔴 Recording gesture... perform a gesture now!
                  </span>
                )}
                {recordingKeys !== null && (
                  <span style={{ marginLeft: "15px", color: "#f38ba8" }}>
                    🔴 Recording keys... press a key combination!
                  </span>
                )}
              </div>
            </div>
          )}
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
              {log.app && <span className="app">[{log.app}]</span>}
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
