import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "../hooks/useTauriEvent";

interface IndexationTabProps {
  projectId: string;
}

interface IndexationProgress {
  projectId: string;
  checked: number;
  total: number;
  currentUrl: string;
  status: string;
}

export default function IndexationTab({ projectId }: IndexationTabProps) {
  const [unverifiedCount, setUnverifiedCount] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<IndexationProgress | null>(null);

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
  }, [projectId]);

  useTauriEvent<IndexationProgress>("indexation-progress", (payload) => {
    if (payload.projectId !== projectId) return;
    setProgress(payload);

    if (payload.status === "done" || payload.status === "stopped") {
      setRunning(false);
      // Refresh count after completion
      invoke<number>("get_unverified_count", { projectId }).then((count) => {
        setUnverifiedCount(count);
      });
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

  return (
    <div className="space-y-6">
      {/* Summary bar */}
      <div className="bg-white/5 rounded-lg p-4 border border-white/10">
        {loading ? (
          <div className="flex items-center gap-2 text-gray-400">
            <span className="w-4 h-4 border-2 border-white/20 border-t-white/60 rounded-full animate-spin inline-block" />
            Loading...
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-sm text-gray-200">
              <span className="font-semibold text-white">
                {unverifiedCount ?? 0}
              </span>{" "}
              URL{unverifiedCount !== 1 ? "s" : ""} need indexation verification
            </p>
            <p className="text-xs text-gray-500">
              This will use approximately{" "}
              <span className="text-gray-400">{unverifiedCount ?? 0}</span>{" "}
              serper credits
            </p>
          </div>
        )}
      </div>

      {/* Action buttons */}
      <div className="bg-white/5 rounded-lg p-4 border border-white/10">
        <div className="flex gap-3">
          <button
            onClick={handleVerify}
            disabled={running || !unverifiedCount}
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
            {progress.status === "stopped" && (
              <>
                <span className="w-2 h-2 rounded-full bg-yellow-400 inline-block" />
                <span className="text-xs text-yellow-400">Stopped</span>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
