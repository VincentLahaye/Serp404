import { useState, useEffect } from "react";
import { useParams, Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import CollectionTab from "../components/CollectionTab";
import IndexationTab from "../components/IndexationTab";

type Tab = "collection" | "indexation" | "audit";

interface ProjectData {
  id: string;
  domain: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

const TABS: { key: Tab; label: string }[] = [
  { key: "collection", label: "Collection" },
  { key: "indexation", label: "Indexation" },
  { key: "audit", label: "Audit" },
];

export default function Project() {
  const { id } = useParams();
  const [project, setProject] = useState<ProjectData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("collection");

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
  }, [id]);

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
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`px-4 py-3 text-sm font-medium cursor-pointer transition-colors ${
              activeTab === tab.key
                ? "text-white border-b-2 border-blue-500"
                : "text-gray-400 hover:text-gray-200"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="mt-6">
        {activeTab === "collection" && (
          <CollectionTab projectId={project.id} />
        )}
        {activeTab === "indexation" && (
          <IndexationTab projectId={project.id} />
        )}
        {activeTab === "audit" && (
          <div className="bg-white/5 rounded-lg p-6 border border-white/10 text-center">
            <p className="text-gray-400 text-sm">
              Audit features coming soon.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
