import { useState, useMemo } from "react";

interface UrlListEntry {
  id: string;
  url: string;
  source: string;
  indexedStatus: string;
}

interface FilterDef {
  key: string;
  label: string;
}

interface UrlListTableProps {
  urls: UrlListEntry[];
  filters: FilterDef[];
  activeFilter: string;
  onFilterChange: (f: string) => void;
  filterField: "source" | "indexedStatus";
  emptyMessage?: string;
}

type SortField = "url" | "source" | "indexedStatus";
type SortDir = "asc" | "desc";

function statusLabel(status: string) {
  switch (status) {
    case "confirmed":
      return (
        <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-green-600/30 text-green-400">
          Confirmed
        </span>
      );
    case "not_indexed":
      return (
        <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-red-600/30 text-red-400">
          Not indexed
        </span>
      );
    case "unknown":
      return (
        <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-gray-600/50 text-gray-300">
          Pending
        </span>
      );
    default:
      return (
        <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-gray-600/50 text-gray-300">
          {status}
        </span>
      );
  }
}

function sourceLabel(source: string) {
  switch (source) {
    case "sitemap":
      return <span className="text-xs text-blue-400">Sitemap</span>;
    case "serper":
      return <span className="text-xs text-purple-400">Serper</span>;
    case "csv":
      return <span className="text-xs text-amber-400">CSV</span>;
    default:
      return <span className="text-xs text-gray-400">{source}</span>;
  }
}

export default function UrlListTable({
  urls,
  filters,
  activeFilter,
  onFilterChange,
  filterField,
  emptyMessage = "No URLs found.",
}: UrlListTableProps) {
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

  const filteredUrls = useMemo(() => {
    if (activeFilter === "all") return urls;
    return urls.filter((u) => u[filterField] === activeFilter);
  }, [urls, activeFilter, filterField]);

  const sortedUrls = useMemo(() => {
    const sorted = [...filteredUrls];
    sorted.sort((a, b) => {
      const cmp = a[sortField].localeCompare(b[sortField]);
      return sortDir === "asc" ? cmp : -cmp;
    });
    return sorted;
  }, [filteredUrls, sortField, sortDir]);

  const visibleUrls = sortedUrls.slice(0, visibleCount);
  const hasMore = sortedUrls.length > visibleCount;

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
        {filters.map((f) => {
          const count =
            f.key === "all"
              ? urls.length
              : urls.filter((u) => u[filterField] === f.key).length;
          return (
            <button
              key={f.key}
              onClick={() => {
                onFilterChange(f.key);
                setVisibleCount(100);
              }}
              className={`px-3 py-1 text-xs font-medium rounded-full transition-colors cursor-pointer ${
                activeFilter === f.key
                  ? "bg-blue-600 text-white"
                  : "bg-white/5 text-gray-400 hover:text-gray-200 hover:bg-white/10"
              }`}
            >
              {f.label}
              <span className="ml-1 opacity-60">{count}</span>
            </button>
          );
        })}
      </div>

      {/* Table */}
      <div className="overflow-x-auto max-h-[480px] overflow-y-auto">
        <table className="w-full text-sm">
          <thead className="bg-[#1a1a2e] text-gray-400 text-xs uppercase tracking-wide sticky top-0 z-10">
            <tr>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200"
                onClick={() => handleSort("url")}
              >
                URL{sortIndicator("url")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200 w-24"
                onClick={() => handleSort("source")}
              >
                Source{sortIndicator("source")}
              </th>
              <th
                className="text-left p-3 cursor-pointer select-none hover:text-gray-200 w-28"
                onClick={() => handleSort("indexedStatus")}
              >
                Indexed{sortIndicator("indexedStatus")}
              </th>
            </tr>
          </thead>
          <tbody>
            {visibleUrls.length === 0 && (
              <tr>
                <td
                  colSpan={3}
                  className="p-6 text-center text-gray-500 text-sm"
                >
                  {urls.length === 0
                    ? emptyMessage
                    : "No URLs match the current filter."}
                </td>
              </tr>
            )}
            {visibleUrls.map((u, i) => (
              <tr
                key={u.id}
                className={`border-b border-white/5 hover:bg-white/5 ${
                  i % 2 === 1 ? "bg-white/[0.02]" : ""
                }`}
              >
                <td className="p-3">
                  <span
                    className="text-gray-200 font-mono text-xs block truncate max-w-[500px]"
                    title={u.url}
                  >
                    {u.url}
                  </span>
                </td>
                <td className="p-3">{sourceLabel(u.source)}</td>
                <td className="p-3">{statusLabel(u.indexedStatus)}</td>
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
            Show more ({sortedUrls.length - visibleCount} remaining)
          </button>
        </div>
      )}

      {/* Result count */}
      {filteredUrls.length > 0 && (
        <div className="px-3 pb-2 text-xs text-gray-500">
          Showing {Math.min(visibleCount, sortedUrls.length)} of{" "}
          {sortedUrls.length} URL{sortedUrls.length !== 1 ? "s" : ""}
        </div>
      )}
    </div>
  );
}

export type { UrlListEntry };
