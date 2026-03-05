import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function Settings() {
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<"success" | "error" | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "serper_api_key" }).then(
      (val) => {
        if (val) setApiKey(val);
      },
    );
  }, []);

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const ok = await invoke<boolean>("test_serper_key", { apiKey });
      setTestResult(ok ? "success" : "error");
    } catch {
      setTestResult("error");
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    setSaved(false);
    try {
      await invoke("set_setting", { key: "serper_api_key", value: apiKey });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch {
      // silently fail for now
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-8 max-w-2xl">
      <h1 className="text-2xl font-bold mb-8">Settings</h1>

      <div className="space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            serper.dev API Key
          </label>

          <div className="relative">
            <input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setTestResult(null);
                setSaved(false);
              }}
              placeholder="Enter your serper.dev API key"
              className="w-full px-4 py-2 pr-10 bg-white/5 border border-white/10 rounded-lg text-gray-100 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50"
            />
            <button
              type="button"
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-200 transition-colors"
              aria-label={showKey ? "Hide API key" : "Show API key"}
            >
              {showKey ? (
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                  <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                  <line x1="1" y1="1" x2="23" y2="23" />
                  <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24" />
                </svg>
              ) : (
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
              )}
            </button>
          </div>

          <div className="flex items-center gap-3 mt-4">
            <button
              onClick={handleTest}
              disabled={testing || !apiKey}
              className="px-4 py-2 text-sm font-medium rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 hover:text-white disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              {testing ? "Testing..." : "Test Key"}
            </button>
            <button
              onClick={handleSave}
              disabled={saving || !apiKey}
              className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              {saving ? "Saving..." : "Save"}
            </button>

            {testResult === "success" && (
              <span className="text-sm text-green-400">Key is valid</span>
            )}
            {testResult === "error" && (
              <span className="text-sm text-red-400">
                Invalid key or request failed
              </span>
            )}
            {saved && (
              <span className="text-sm text-green-400">Saved</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
