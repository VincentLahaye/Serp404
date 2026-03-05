import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "../hooks/useTauriEvent";
import UrlListTable from "./UrlListTable";
import type { UrlListEntry } from "./UrlListTable";

interface CollectionTabProps {
  projectId: string;
  onStatsChange?: () => void;
}

interface CollectionProgress {
  projectId: string;
  source: string;
  urlsFound: number;
  status: string;
  message: string;
}

interface CsvColumn {
  index: number;
  name: string;
  urlCount: number;
  sampleUrl: string | null;
}

interface SourceStatus {
  source: string;
  urlsFound: number;
  status: string; // "running" | "done" | "error"
}

export default function CollectionTab({ projectId, onStatsChange }: CollectionTabProps) {
  const [hasSerperKey, setHasSerperKey] = useState(false);
  const [sources, setSources] = useState<Record<string, SourceStatus>>({});
  const [logs, setLogs] = useState<string[]>([]);
  const [totalUrls, setTotalUrls] = useState(0);

  // Persisted counts from DB
  const [dbCounts, setDbCounts] = useState<Record<string, number> | null>(null);
  const [dbTotal, setDbTotal] = useState(0);

  // URL list
  const [urlList, setUrlList] = useState<UrlListEntry[]>([]);
  const [sourceFilter, setSourceFilter] = useState("all");

  // CSV state
  const [csvColumns, setCsvColumns] = useState<CsvColumn[] | null>(null);
  const [csvContent, setCsvContent] = useState("");
  const [selectedColumn, setSelectedColumn] = useState<number>(0);
  const [csvLoading, setCsvLoading] = useState(false);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const logEndRef = useRef<HTMLDivElement>(null);

  const fetchUrlList = useCallback(() => {
    invoke<UrlListEntry[]>("get_project_urls", { projectId, source: null, indexedStatus: null })
      .then(setUrlList)
      .catch(() => {});
  }, [projectId]);

  // Load persisted URL counts and list from DB
  useEffect(() => {
    invoke<Record<string, number>>("get_url_counts_by_source", { projectId })
      .then((counts) => {
        setDbCounts(counts);
        const total = Object.values(counts).reduce((sum, c) => sum + c, 0);
        setDbTotal(total);
      })
      .catch(() => {});
    fetchUrlList();
  }, [projectId, fetchUrlList]);

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "serper_api_key" }).then(
      (val) => {
        setHasSerperKey(!!val);
      },
    );
  }, []);

  useTauriEvent<CollectionProgress>("collection-progress", (payload) => {
    if (payload.projectId !== projectId) return;

    setSources((prev) => ({
      ...prev,
      [payload.source]: {
        source: payload.source,
        urlsFound: payload.urlsFound,
        status: payload.status,
      },
    }));

    if (payload.message) {
      setLogs((prev) => {
        const next = [...prev, payload.message];
        return next.slice(-20);
      });
    }

    // Refresh DB counts when a source finishes
    if (payload.status === "done" || payload.status === "error") {
      invoke<Record<string, number>>("get_url_counts_by_source", { projectId })
        .then((counts) => {
          setDbCounts(counts);
          const total = Object.values(counts).reduce((sum, c) => sum + c, 0);
          setDbTotal(total);
        })
        .catch(() => {});
      fetchUrlList();
      onStatsChange?.();
    }
  });

  // Recompute session total whenever sources change
  useEffect(() => {
    const total = Object.values(sources).reduce(
      (sum, s) => sum + s.urlsFound,
      0,
    );
    setTotalUrls(total);
  }, [sources]);

  // Auto-scroll logs
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const handleSitemap = useCallback(async () => {
    try {
      setSources((prev) => ({
        ...prev,
        sitemap: { source: "sitemap", urlsFound: 0, status: "running" },
      }));
      await invoke("collect_from_sitemap", { projectId });
    } catch (err) {
      console.error("Sitemap collection failed:", err);
      setSources((prev) => ({
        ...prev,
        sitemap: {
          ...prev.sitemap,
          status: "error",
        },
      }));
    }
  }, [projectId]);

  const handleSerper = useCallback(async () => {
    try {
      setSources((prev) => ({
        ...prev,
        serper: { source: "serper", urlsFound: 0, status: "running" },
      }));
      await invoke("collect_from_serper", { projectId });
    } catch (err) {
      console.error("Serper collection failed:", err);
      setSources((prev) => ({
        ...prev,
        serper: {
          ...prev.serper,
          status: "error",
        },
      }));
    }
  }, [projectId]);

  const handleCsvClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;

      setCsvLoading(true);
      setCsvColumns(null);

      const reader = new FileReader();
      reader.onload = async () => {
        const content = reader.result as string;
        setCsvContent(content);

        try {
          const columns = await invoke<CsvColumn[]>("detect_csv_columns", {
            content,
          });
          setCsvColumns(columns);
          if (columns.length > 0) {
            setSelectedColumn(columns[0].index);
          }
        } catch (err) {
          console.error("Failed to detect CSV columns:", err);
        } finally {
          setCsvLoading(false);
        }
      };
      reader.readAsText(file);

      // Reset file input so same file can be re-selected
      e.target.value = "";
    },
    [],
  );

  const handleCsvConfirm = useCallback(async () => {
    try {
      setSources((prev) => ({
        ...prev,
        csv: { source: "csv", urlsFound: 0, status: "running" },
      }));
      setCsvColumns(null);
      await invoke("collect_from_csv", {
        projectId,
        content: csvContent,
        columnIndex: selectedColumn,
      });
    } catch (err) {
      console.error("CSV collection failed:", err);
      setSources((prev) => ({
        ...prev,
        csv: {
          ...prev.csv,
          status: "error",
        },
      }));
    }
  }, [projectId, csvContent, selectedColumn]);

  const handleCsvCancel = useCallback(() => {
    setCsvColumns(null);
    setCsvContent("");
  }, []);

  function statusIcon(status: string) {
    if (status === "running") {
      return (
        <span className="w-4 h-4 border-2 border-blue-400/30 border-t-blue-400 rounded-full animate-spin inline-block" />
      );
    }
    if (status === "done") {
      return <span className="text-green-400 text-sm font-bold">&#10003;</span>;
    }
    if (status === "error") {
      return <span className="text-red-400 text-sm font-bold">&#10007;</span>;
    }
    return null;
  }

  const sourceLabels: Record<string, string> = {
    sitemap: "Sitemap",
    serper: "Serper",
    csv: "CSV",
  };

  return (
    <div className="space-y-6">
      {/* Persisted summary from DB */}
      {dbCounts && dbTotal > 0 && Object.keys(sources).length === 0 && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <p className="text-sm text-gray-200">
            <span className="font-semibold text-white">{dbTotal}</span> URL
            {dbTotal !== 1 ? "s" : ""} collected
          </p>
          <div className="flex gap-4 mt-2">
            {Object.entries(dbCounts).map(([source, count]) => (
              <span key={source} className="text-xs text-gray-400">
                <span className="text-gray-300 font-medium">{count}</span>{" "}
                from {sourceLabels[source] ?? source}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Source buttons */}
      <div className="bg-white/5 rounded-lg p-4 border border-white/10">
        <h3 className="text-sm font-medium text-gray-300 mb-3">
          URL Sources
        </h3>
        <div className="flex flex-wrap gap-3">
          <button
            onClick={handleSitemap}
            className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 transition-colors"
          >
            Fetch Sitemap
          </button>

          <div className="relative group">
            <button
              onClick={handleSerper}
              disabled={!hasSerperKey}
              className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              Search via serper
            </button>
            {!hasSerperKey && (
              <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 text-xs bg-gray-800 text-gray-300 rounded-lg whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
                Configure API key in Settings
              </div>
            )}
          </div>

          <button
            onClick={handleCsvClick}
            className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 transition-colors"
          >
            Upload CSV
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv"
            onChange={handleFileChange}
            className="hidden"
          />
        </div>
      </div>

      {/* CSV column selection */}
      {csvLoading && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <div className="flex items-center gap-2 text-gray-300">
            <span className="w-4 h-4 border-2 border-blue-400/30 border-t-blue-400 rounded-full animate-spin inline-block" />
            Detecting CSV columns...
          </div>
        </div>
      )}

      {csvColumns && csvColumns.length > 0 && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <h3 className="text-sm font-medium text-gray-300 mb-3">
            Select URL column
          </h3>
          <div className="space-y-2">
            {csvColumns.map((col) => (
              <label
                key={col.index}
                className="flex items-center gap-3 p-2 rounded-lg hover:bg-white/5 cursor-pointer"
              >
                <input
                  type="radio"
                  name="csv-column"
                  checked={selectedColumn === col.index}
                  onChange={() => setSelectedColumn(col.index)}
                  className="accent-blue-500"
                />
                <div>
                  <span className="text-sm text-gray-200 font-medium">
                    {col.name}
                  </span>
                  <span className="text-xs text-gray-500 ml-2 font-mono">
                    {col.sampleUrl ?? ""}
                  </span>
                </div>
              </label>
            ))}
          </div>
          <div className="flex gap-3 mt-4">
            <button
              onClick={handleCsvConfirm}
              className="px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-500 transition-colors"
            >
              Import
            </button>
            <button
              onClick={handleCsvCancel}
              className="px-4 py-2 text-sm font-medium rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Progress per source */}
      {Object.keys(sources).length > 0 && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <h3 className="text-sm font-medium text-gray-300 mb-3">Progress</h3>
          <div className="space-y-2">
            {Object.values(sources).map((s) => (
              <div
                key={s.source}
                className="flex items-center justify-between py-1"
              >
                <div className="flex items-center gap-2">
                  {statusIcon(s.status)}
                  <span className="text-sm text-gray-200 capitalize">
                    {s.source}
                  </span>
                </div>
                <span className="text-sm text-gray-400 font-mono">
                  {s.urlsFound} URL{s.urlsFound !== 1 ? "s" : ""}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Log area */}
      {logs.length > 0 && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <h3 className="text-sm font-medium text-gray-300 mb-3">
            Recent URLs
          </h3>
          <div className="max-h-60 overflow-y-auto font-mono text-xs text-gray-400 space-y-0.5">
            {logs.map((log, i) => (
              <div key={i} className="truncate">
                {log}
              </div>
            ))}
            <div ref={logEndRef} />
          </div>
        </div>
      )}

      {/* Summary */}
      {totalUrls > 0 && (
        <div className="bg-white/5 rounded-lg p-4 border border-white/10">
          <p className="text-sm text-gray-200">
            <span className="font-semibold text-white">{totalUrls}</span> total
            URL{totalUrls !== 1 ? "s" : ""} found across all sources
          </p>
        </div>
      )}

      {/* URL list */}
      {urlList.length > 0 && (
        <UrlListTable
          urls={urlList}
          filters={[
            { key: "all", label: "All" },
            { key: "sitemap", label: "Sitemap" },
            { key: "serper", label: "Serper" },
            { key: "csv", label: "CSV" },
          ]}
          activeFilter={sourceFilter}
          onFilterChange={setSourceFilter}
          filterField="source"
          emptyMessage="No URLs collected yet."
        />
      )}
    </div>
  );
}
