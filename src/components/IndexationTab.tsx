import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "../hooks/useTauriEvent";
import UrlListTable from "./UrlListTable";
import type { UrlListEntry } from "./UrlListTable";

interface IndexationTabProps {
  projectId: string;
  stats: {
    totalUrls: number;
    confirmedIndexed: number;
    notIndexed: number;
    unknownStatus: number;
  } | null;
  onStatsChange?: () => void;
}

interface IndexationProgress {
  projectId: string;
  checked: number;
  total: number;
  currentUrl: string;
  status: string;
}

export default function IndexationTab({ projectId, stats, onStatsChange }: IndexationTabProps) {
  const [unverifiedCount, setUnverifiedCount] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<IndexationProgress | null>(null);
  const [urlList, setUrlList] = useState<UrlListEntry[]>([]);
  const [statusFilter, setStatusFilter] = useState("all");

  const fetchUrlList = useCallback(() => {
    invoke<UrlListEntry[]>("get_project_urls", { projectId, source: null, indexedStatus: null })
      .then(setUrlList)
      .catch(() => {});
  }, [projectId]);

  useEffect(() => {
    setLoading(true);
    invoke<number>("get_unverified_count", { projectId })
      .then((count) => {
        setUnverifiedCount(count);
      })
      .catch((err) => {
        console.error("Failed to get unverified count:", err);
      })
      .finally(() => {
        setLoading(false);
      });
    fetchUrlList();
  }, [projectId, fetchUrlList]);

  useTauriEvent<IndexationProgress>("indexation-progress", (payload) => {
    if (payload.projectId !== projectId) return;
    setProgress(payload);

    if (payload.status === "done" || payload.status === "cancelled") {
      setRunning(false);
      // Refresh count and URL list after completion
      invoke<number>("get_unverified_count", { projectId }).then((count) => {
        setUnverifiedCount(count);
      });
      fetchUrlList();
      onStatsChange?.();
    }
  });

  const handleVerify = useCallback(async () => {
    try {
      setRunning(true);
      setProgress(null);
      await invoke("verify_indexation", { projectId });
    } catch (err) {
      console.error("Indexation verification failed:", err);
      setRunning(false);
    }
  }, [projectId]);

  const handleStop = useCallback(async () => {
    try {
      await invoke("stop_indexation", { projectId });
    } catch (err) {
      console.error("Failed to stop indexation:", err);
    }
  }, [projectId]);

  const progressPercent =
    progress && progress.total > 0
      ? Math.round((progress.checked / progress.total) * 100)
      : 0;

  const hasUrls = stats && stats.totalUrls > 0;
  const allConfirmed = stats && stats.unknownStatus === 0 && stats.confirmedIndexed > 0;

  return (
    <div className="space-y-6">
      {/* Status overview */}
      {stats && hasUrls && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <h3 className="text-sm font-medium text-gray-300 mb-3">
            Indexation Status
          </h3>
          <div className="grid grid-cols-3 gap-4">
            <div className="text-center">
              <p className="text-2xl font-bold text-green-400">
                {stats.confirmedIndexed}
              </p>
              <p className="text-xs text-gray-400 mt-1">Confirmed</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-red-400">
                {stats.notIndexed}
              </p>
              <p className="text-xs text-gray-400 mt-1">Not indexed</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-gray-400">
                {stats.unknownStatus}
              </p>
              <p className="text-xs text-gray-400 mt-1">Pending</p>
            </div>
          </div>

          {allConfirmed && (
            <p className="text-xs text-gray-500 mt-4 text-center">
              All URLs are already confirmed as indexed.
              {stats.confirmedIndexed > 0 && stats.notIndexed === 0 && (
                <span className="block mt-1 text-gray-400">
                  URLs collected via Serper are automatically marked as indexed.
                </span>
              )}
            </p>
          )}
        </div>
      )}

      {/* Empty state when no URLs at all */}
      {stats && !hasUrls && !loading && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <p className="text-sm text-gray-400 text-center py-4">
            No URLs collected yet. Go to the Collection tab to add URLs first.
          </p>
        </div>
      )}

      {/* Verification section */}
      {loading ? (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <div className="flex items-center gap-2 text-gray-400">
            <span className="w-4 h-4 border-2 border-white/20 border-t-white/60 rounded-full animate-spin inline-block" />
            Loading...
          </div>
        </div>
      ) : unverifiedCount !== null && unverifiedCount > 0 ? (
        <>
          {/* Summary bar */}
          <div className="bg-white/5 rounded-lg p-4 border border-white/10">
            <div className="space-y-2">
              <p className="text-sm text-gray-200">
                <span className="font-semibold text-white">
                  {unverifiedCount}
                </span>{" "}
                URL{unverifiedCount !== 1 ? "s" : ""} need indexation verification
              </p>
              <p className="text-xs text-gray-500">
                This will use approximately{" "}
                <span className="text-gray-400">{unverifiedCount}</span>{" "}
                serper credits
              </p>
            </div>
          </div>

          {/* Action buttons */}
          <div className="bg-white/5 rounded-lg p-4 border border-white/10">
            <div className="flex gap-3">
              <button
                onClick={handleVerify}
                disabled={running}
                className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                {running ? "Verifying..." : "Verify Indexation"}
              </button>
              <button
                onClick={handleStop}
                disabled={!running}
                className="px-4 py-2 text-sm font-medium rounded-lg bg-red-600/80 text-white hover:bg-red-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Stop
              </button>
            </div>
          </div>
        </>
      ) : null}

      {/* Progress */}
      {progress && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <h3 className="text-sm font-medium text-gray-300 mb-3">Progress</h3>

          {/* Progress bar */}
          <div className="w-full h-2 bg-white/10 rounded-full overflow-hidden mb-3">
            <div
              className="h-full bg-blue-500 rounded-full transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>

          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-400">
              {progress.checked} / {progress.total} checked
            </span>
            <span className="text-gray-400">{progressPercent}%</span>
          </div>

          {/* Current URL */}
          {progress.currentUrl && (
            <p className="mt-2 text-xs text-gray-500 font-mono truncate">
              {progress.currentUrl}
            </p>
          )}

          {/* Status indicator */}
          <div className="mt-2 flex items-center gap-2">
            {progress.status === "running" && (
              <>
                <span className="w-3 h-3 border-2 border-blue-400/30 border-t-blue-400 rounded-full animate-spin inline-block" />
                <span className="text-xs text-blue-400">Running</span>
              </>
            )}
            {progress.status === "done" && (
              <>
                <span className="text-green-400 text-xs font-bold">
                  &#10003;
                </span>
                <span className="text-xs text-green-400">Complete</span>
              </>
            )}
            {progress.status === "cancelled" && (
              <>
                <span className="w-2 h-2 rounded-full bg-yellow-400 inline-block" />
                <span className="text-xs text-yellow-400">Stopped</span>
              </>
            )}
          </div>
        </div>
      )}

      {/* URL list */}
      {urlList.length > 0 && (
        <UrlListTable
          urls={urlList}
          filters={[
            { key: "all", label: "All" },
            { key: "confirmed", label: "Confirmed" },
            { key: "not_indexed", label: "Not indexed" },
            { key: "unknown", label: "Pending" },
          ]}
          activeFilter={statusFilter}
          onFilterChange={setStatusFilter}
          filterField="indexedStatus"
          emptyMessage="No URLs to display."
        />
      )}
    </div>
  );
}
