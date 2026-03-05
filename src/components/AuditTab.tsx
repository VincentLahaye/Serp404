import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "../hooks/useTauriEvent";
import ConcurrencySlider from "./ConcurrencySlider";
import StatsBar from "./StatsBar";
import type { AuditStats } from "./StatsBar";
import ResultsTable from "./ResultsTable";
import type { UrlResult } from "./ResultsTable";

interface AuditTabProps {
  projectId: string;
}

interface AuditProgress {
  projectId: string;
  checked: number;
  total: number;
  currentUrl: string;
  status: string;
  stats: AuditStats;
}

interface ProjectStats {
  totalUrls: number;
  confirmedIndexed: number;
  notIndexed: number;
  unknownStatus: number;
  checked: number;
  okCount: number;
  redirectCount: number;
  notFoundCount: number;
  errorCount: number;
  emptyTitleCount: number;
  slowCount: number;
}

type AuditState = "idle" | "running" | "paused" | "done";

const DEFAULT_STATS: AuditStats = {
  okCount: 0,
  redirectCount: 0,
  notFoundCount: 0,
  errorCount: 0,
  emptyTitleCount: 0,
  slowCount: 0,
};

export default function AuditTab({ projectId }: AuditTabProps) {
  const [auditState, setAuditState] = useState<AuditState>("idle");
  const [concurrency, setConcurrency] = useState(5);
  const [stats, setStats] = useState<AuditStats>(DEFAULT_STATS);
  const [results, setResults] = useState<UrlResult[]>([]);
  const [filter, setFilter] = useState("all");
  const [checked, setChecked] = useState(0);
  const [total, setTotal] = useState(0);
  const [currentUrl, setCurrentUrl] = useState("");

  // Fetch checked URLs from DB and map to UrlResult[]
  const fetchCheckedUrls = useCallback(() => {
    invoke<
      Array<{
        url: string;
        httpStatus: number | null;
        responseTimeMs: number | null;
        title: string | null;
        redirectChain: string | null;
        error: string | null;
      }>
    >("get_checked_urls", { projectId, filter: null })
      .then((entries) => {
        setResults(
          entries.map((e) => ({
            url: e.url,
            httpStatus: e.httpStatus,
            responseTimeMs: e.responseTimeMs,
            title: e.title,
            redirectChain: e.redirectChain,
            error: e.error,
          })),
        );
      })
      .catch((err) => {
        console.error("Failed to load checked URLs:", err);
      });
  }, [projectId]);

  // Load initial stats and results from DB on mount
  useEffect(() => {
    invoke<ProjectStats>("get_project_stats", { projectId })
      .then((s) => {
        if (s.checked > 0) {
          setStats({
            okCount: s.okCount,
            redirectCount: s.redirectCount,
            notFoundCount: s.notFoundCount,
            errorCount: s.errorCount,
            emptyTitleCount: s.emptyTitleCount,
            slowCount: s.slowCount,
          });
          setChecked(s.checked);
        }
      })
      .catch((err) => {
        console.error("Failed to load project stats:", err);
      });

    fetchCheckedUrls();
  }, [projectId, fetchCheckedUrls]);

  // Listen to real-time audit progress events
  useTauriEvent<AuditProgress>("audit-progress", (payload) => {
    if (payload.projectId !== projectId) return;

    setChecked(payload.checked);
    setTotal(payload.total);
    setCurrentUrl(payload.currentUrl);
    setStats(payload.stats);

    // Map backend status to UI state
    switch (payload.status) {
      case "running":
        setAuditState("running");
        break;
      case "paused":
        setAuditState("paused");
        break;
      case "done":
        setAuditState("done");
        fetchCheckedUrls();
        break;
      case "cancelled":
        setAuditState("done");
        fetchCheckedUrls();
        break;
    }
  });

  const handleStart = useCallback(async () => {
    try {
      setResults([]);
      setStats(DEFAULT_STATS);
      setChecked(0);
      setTotal(0);
      setCurrentUrl("");
      setAuditState("running");
      await invoke("start_audit", { projectId, concurrency });
    } catch (err) {
      console.error("Failed to start audit:", err);
      setAuditState("idle");
    }
  }, [projectId, concurrency]);

  const handlePause = useCallback(async () => {
    try {
      await invoke("pause_audit", { projectId });
      setAuditState("paused");
    } catch (err) {
      console.error("Failed to pause audit:", err);
    }
  }, [projectId]);

  const handleResume = useCallback(async () => {
    try {
      await invoke("resume_audit", { projectId });
      setAuditState("running");
    } catch (err) {
      console.error("Failed to resume audit:", err);
    }
  }, [projectId]);

  const handleStop = useCallback(async () => {
    try {
      await invoke("stop_audit", { projectId });
    } catch (err) {
      console.error("Failed to stop audit:", err);
    }
  }, [projectId]);

  const handleConcurrencyChange = useCallback(
    (val: number) => {
      setConcurrency(val);
      if (auditState === "running" || auditState === "paused") {
        invoke("update_concurrency", { projectId, concurrency: val }).catch(
          (err) => console.error("Failed to update concurrency:", err),
        );
      }
    },
    [projectId, auditState],
  );

  const handleExportCsv = useCallback(async () => {
    try {
      const csv = await invoke<string>("export_csv", {
        projectId,
        filter: filter === "all" ? null : filter,
      });
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `audit-${projectId.slice(0, 8)}-${filter}.csv`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error("Failed to export CSV:", err);
    }
  }, [projectId, filter]);

  const progressPercent =
    total > 0 ? Math.round((checked / total) * 100) : 0;

  const isRunningOrPaused = auditState === "running" || auditState === "paused";

  return (
    <div className="space-y-4">
      {/* Controls row */}
      <div className="bg-white/5 rounded-lg p-4 border border-white/10">
        <div className="flex items-center justify-between flex-wrap gap-3">
          <div className="flex items-center gap-4">
            <ConcurrencySlider
              value={concurrency}
              onChange={handleConcurrencyChange}
              disabled={auditState === "done"}
            />

            <div className="flex gap-2">
              {(auditState === "idle" || auditState === "done") && (
                <button
                  onClick={handleStart}
                  className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 transition-colors cursor-pointer"
                >
                  Start Audit
                </button>
              )}
              {auditState === "running" && (
                <button
                  onClick={handlePause}
                  className="px-4 py-2 text-sm font-medium rounded-lg bg-yellow-600 text-white hover:bg-yellow-500 transition-colors cursor-pointer"
                >
                  Pause
                </button>
              )}
              {auditState === "paused" && (
                <button
                  onClick={handleResume}
                  className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 transition-colors cursor-pointer"
                >
                  Resume
                </button>
              )}
              {isRunningOrPaused && (
                <button
                  onClick={handleStop}
                  className="px-4 py-2 text-sm font-medium rounded-lg bg-red-600/80 text-white hover:bg-red-500 transition-colors cursor-pointer"
                >
                  Stop
                </button>
              )}
            </div>
          </div>

          <button
            onClick={handleExportCsv}
            className="px-4 py-2 text-sm font-medium rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 transition-colors cursor-pointer"
          >
            Export CSV
          </button>
        </div>
      </div>

      {/* Progress bar */}
      {(isRunningOrPaused || (auditState === "done" && total > 0)) && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-300">
              {checked} / {total} URLs checked
            </span>
            <span className="text-sm text-gray-400">{progressPercent}%</span>
          </div>
          <div className="w-full h-2 bg-white/10 rounded-full overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-300 ${
                auditState === "done" ? "bg-green-500" : "bg-blue-500"
              }`}
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          {currentUrl && auditState === "running" && (
            <p className="mt-2 text-xs text-gray-500 font-mono truncate">
              {currentUrl}
            </p>
          )}
          {auditState === "paused" && (
            <div className="mt-2 flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-yellow-400 inline-block" />
              <span className="text-xs text-yellow-400">Paused</span>
            </div>
          )}
          {auditState === "done" && total > 0 && (
            <div className="mt-2 flex items-center gap-2">
              <span className="text-green-400 text-xs font-bold">
                &#10003;
              </span>
              <span className="text-xs text-green-400">Complete</span>
            </div>
          )}
        </div>
      )}

      {/* Stats bar */}
      <StatsBar stats={stats} />

      {/* Results table */}
      <ResultsTable
        results={results}
        filter={filter}
        onFilterChange={setFilter}
      />
    </div>
  );
}
