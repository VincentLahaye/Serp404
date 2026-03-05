import { useState, useEffect, useCallback } from "react";
import { useParams, Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import CollectionTab from "../components/CollectionTab";
import IndexationTab from "../components/IndexationTab";
import AuditTab from "../components/AuditTab";

type Tab = "collection" | "indexation" | "audit";

interface ProjectData {
  id: string;
  domain: string;
  status: string;
  createdAt: string;
  updatedAt: string;
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

const TABS: { key: Tab; label: string }[] = [
  { key: "collection", label: "Collection" },
  { key: "indexation", label: "Indexation" },
  { key: "audit", label: "Audit" },
];

function tabBadge(tab: Tab, stats: ProjectStats | null): string | null {
  if (!stats) return null;
  switch (tab) {
    case "collection":
      return stats.totalUrls > 0 ? String(stats.totalUrls) : null;
    case "indexation":
      return stats.unknownStatus > 0 ? String(stats.unknownStatus) : null;
    case "audit":
      return stats.checked > 0 ? String(stats.checked) : null;
  }
}

export default function Project() {
  const { id } = useParams();
  const [project, setProject] = useState<ProjectData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("collection");
  const [stats, setStats] = useState<ProjectStats | null>(null);

  const refreshStats = useCallback(() => {
    if (!id) return;
    invoke<ProjectStats>("get_project_stats", { projectId: id }).then(setStats).catch(() => {});
  }, [id]);

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    setError(null);
    invoke<ProjectData>("get_project", { id })
      .then((data) => {
        setProject(data);
      })
      .catch((err) => {
        setError(String(err));
      })
      .finally(() => {
        setLoading(false);
      });
    refreshStats();
  }, [id, refreshStats]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="w-6 h-6 border-2 border-white/20 border-t-white/60 rounded-full animate-spin" />
      </div>
    );
  }

  if (error || !project) {
    return (
      <div className="p-8">
        <Link
          to="/"
          className="text-sm text-gray-400 hover:text-gray-200 transition-colors"
        >
          &larr; Back to projects
        </Link>
        <p className="mt-4 text-red-400">{error ?? "Project not found"}</p>
      </div>
    );
  }

  return (
    <div className="p-8 max-w-6xl mx-auto">
      {/* Header */}
      <div className="mb-6">
        <Link
          to="/"
          className="text-sm text-gray-400 hover:text-gray-200 transition-colors"
        >
          &larr; Back to projects
        </Link>
        <h1 className="text-2xl font-bold text-gray-100 mt-2">
          {project.domain}
        </h1>
      </div>

      {/* Tab bar */}
      <div className="flex border-b border-white/10">
        {TABS.map((tab) => {
          const badge = tabBadge(tab.key, stats);
          return (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-4 py-3 text-sm font-medium cursor-pointer transition-colors flex items-center gap-2 ${
                activeTab === tab.key
                  ? "text-white border-b-2 border-blue-500"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              {tab.label}
              {badge && (
                <span className="text-xs bg-white/10 text-gray-300 px-1.5 py-0.5 rounded-full font-mono">
                  {badge}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Tab content */}
      <div className="mt-6">
        {activeTab === "collection" && (
          <CollectionTab projectId={project.id} onStatsChange={refreshStats} />
        )}
        {activeTab === "indexation" && (
          <IndexationTab
            projectId={project.id}
            stats={stats}
            onStatsChange={refreshStats}
          />
        )}
        {activeTab === "audit" && (
          <AuditTab projectId={project.id} />
        )}
      </div>
    </div>
  );
}
