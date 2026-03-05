import { useState, useMemo } from "react";

interface UrlResult {
  url: string;
  httpStatus: number | null;
  responseTimeMs: number | null;
  title: string | null;
  redirectChain: string | null;
  error: string | null;
}

interface ResultsTableProps {
  results: UrlResult[];
  filter: string;
  onFilterChange: (f: string) => void;
}

type SortField = "url" | "httpStatus" | "responseTimeMs" | "title" | "redirectChain";
type SortDir = "asc" | "desc";

const FILTERS = [
  { key: "all", label: "All" },
  { key: "404", label: "404" },
  { key: "redirects", label: "Redirects" },
  { key: "empty_title", label: "Empty Titles" },
  { key: "slow", label: "Slow" },
] as const;

function statusBadge(status: number | null, error: string | null) {
  if (status === null || error) {
    return (
      <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-gray-600/50 text-gray-300">
        ERR
      </span>
    );
  }
  if (status >= 200 && status < 300) {
    return (
      <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-green-600/30 text-green-400">
        {status}
      </span>
    );
  }
  if (status >= 300 && status < 400) {
    return (
      <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-yellow-600/30 text-yellow-400">
        {status}
      </span>
    );
  }
  if (status === 404) {
    return (
      <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-red-600/30 text-red-400">
        404
      </span>
    );
  }
  if (status >= 400) {
    return (
      <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-red-600/30 text-red-400">
        {status}
      </span>
    );
  }
  return (
    <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-gray-600/50 text-gray-300">
      {status}
    </span>
  );
}

export default function ResultsTable({
  results,
  filter,
  onFilterChange,
}: ResultsTableProps) {
  const [sortField, setSortField] = useState<SortField>("url");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [visibleCount, setVisibleCount] = useState(100);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortDir("asc");
    }
  };

  const filteredResults = useMemo(() => {
    switch (filter) {
      case "404":
        return results.filter((r) => r.httpStatus === 404);
      case "redirects":
        return results.filter(
          (r) =>
            r.httpStatus !== null &&
            r.httpStatus >= 300 &&
            r.httpStatus < 400,
        );
      case "empty_title":
        return results.filter(
          (r) => r.httpStatus === 200 && (!r.title || r.title.trim() === ""),
        );
      case "slow":
        return results.filter(
          (r) => r.responseTimeMs !== null && r.responseTimeMs > 2000,
        );
      default:
        return results;
    }
  }, [results, filter]);

  const sortedResults = useMemo(() => {
    const sorted = [...filteredResults];
    sorted.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case "url":
          cmp = a.url.localeCompare(b.url);
          break;
        case "httpStatus":
          cmp = (a.httpStatus ?? -1) - (b.httpStatus ?? -1);
          break;
        case "responseTimeMs":
          cmp = (a.responseTimeMs ?? -1) - (b.responseTimeMs ?? -1);
          break;
        case "title":
          cmp = (a.title ?? "").localeCompare(b.title ?? "");
          break;
        case "redirectChain":
          cmp = (a.redirectChain ?? "").localeCompare(b.redirectChain ?? "");
          break;
      }
      return sortDir === "asc" ? cmp : -cmp;
    });
    return sorted;
  }, [filteredResults, sortField, sortDir]);

  const visibleResults = sortedResults.slice(0, visibleCount);
  const hasMore = sortedResults.length > visibleCount;

  function sortIndicator(field: SortField) {
    if (sortField !== field) return null;
    return (
      <span className="ml-1 text-blue-400">
        {sortDir === "asc" ? "\u2191" : "\u2193"}
      </span>
    );
  }

  return (
    <div className="bg-white/5 rounded-lg border border-white/10">
      {/* Filter buttons */}
      <div className="flex gap-2 p-3 border-b border-white/10">
        {FILTERS.map((f) => (
          <button
            key={f.key}
            onClick={() => {
              onFilterChange(f.key);
              setVisibleCount(100);
            }}
            className={`px-3 py-1 text-xs font-medium rounded-full transition-colors cursor-pointer ${
              filter === f.key
                ? "bg-blue-600 text-white"
                : "bg-white/5 text-gray-400 hover:text-gray-200 hover:bg-white/10"
            }`}
          >
            {f.label}
            {f.key !== "all" && (
              <span className="ml-1 opacity-60">
                {f.key === "404"
                  ? results.filter((r) => r.httpStatus === 404).length
                  : f.key === "redirects"
                    ? results.filter(
                        (r) =>
                          r.httpStatus !== null &&
                          r.httpStatus >= 300 &&
                          r.httpStatus < 400,
                      ).length
                    : f.key === "empty_title"
                      ? results.filter(
                          (r) =>
                            r.httpStatus === 200 &&
                            (!r.title || r.title.trim() === ""),
                        ).length
                      : f.key === "slow"
                        ? results.filter(
                            (r) =>
                              r.responseTimeMs !== null &&
                              r.responseTimeMs > 2000,
                          ).length
                        : ""}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Table */}
      <div className="overflow-x-auto max-h-[480px] overflow-y-auto">
        <table className="w-full text-sm">
          <thead className="bg-white/5 text-gray-400 text-xs uppercase tracking-wide sticky top-0">
            <tr>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200"
                onClick={() => handleSort("url")}
              >
                URL{sortIndicator("url")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200 w-20"
                onClick={() => handleSort("httpStatus")}
              >
                Status{sortIndicator("httpStatus")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200 w-24"
                onClick={() => handleSort("responseTimeMs")}
              >
                Time (ms){sortIndicator("responseTimeMs")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200"
                onClick={() => handleSort("title")}
              >
                Title{sortIndicator("title")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200"
                onClick={() => handleSort("redirectChain")}
              >
                Redirects{sortIndicator("redirectChain")}
              </th>
            </tr>
          </thead>
          <tbody>
            {visibleResults.length === 0 && (
              <tr>
                <td
                  colSpan={5}
                  className="p-6 text-center text-gray-500 text-sm"
                >
                  {results.length === 0
                    ? "No results yet. Start an audit to see data here."
                    : "No results match the current filter."}
                </td>
              </tr>
            )}
            {visibleResults.map((r, i) => (
              <tr
                key={r.url}
                className={`border-b border-white/5 hover:bg-white/5 ${
                  i % 2 === 1 ? "bg-white/[0.02]" : ""
                }`}
              >
                <td className="p-3 max-w-[300px]">
                  <span
                    className="text-gray-200 font-mono text-xs block truncate"
                    title={r.url}
                  >
                    {r.url}
                  </span>
                </td>
                <td className="p-3">
                  {statusBadge(r.httpStatus, r.error)}
                </td>
                <td className="p-3 text-gray-300 font-mono text-xs">
                  {r.responseTimeMs !== null ? r.responseTimeMs : "-"}
                </td>
                <td className="p-3 max-w-[200px]">
                  <span
                    className="text-gray-300 text-xs block truncate"
                    title={r.title ?? ""}
                  >
                    {r.title || (
                      <span className="text-gray-600 italic">empty</span>
                    )}
                  </span>
                </td>
                <td className="p-3 max-w-[200px]">
                  <span
                    className="text-gray-400 text-xs block truncate font-mono"
                    title={r.redirectChain ?? ""}
                  >
                    {r.redirectChain || "-"}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Show more */}
      {hasMore && (
        <div className="p-3 border-t border-white/10 text-center">
          <button
            onClick={() => setVisibleCount((c) => c + 100)}
            className="text-xs text-blue-400 hover:text-blue-300 cursor-pointer transition-colors"
          >
            Show more ({sortedResults.length - visibleCount} remaining)
          </button>
        </div>
      )}

      {/* Result count */}
      {filteredResults.length > 0 && (
        <div className="px-3 pb-2 text-xs text-gray-500">
          Showing {Math.min(visibleCount, sortedResults.length)} of{" "}
          {sortedResults.length} result{sortedResults.length !== 1 ? "s" : ""}
        </div>
      )}
    </div>
  );
}

export type { UrlResult };
