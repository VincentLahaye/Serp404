import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import ProjectCard from "../components/ProjectCard";
import NewProjectModal from "../components/NewProjectModal";

interface Project {
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

export default function Home() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [statsMap, setStatsMap] = useState<Record<string, ProjectStats>>({});
  const [showModal, setShowModal] = useState(false);
  const [loading, setLoading] = useState(true);

  const loadProjects = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<Project[]>("list_projects");
      setProjects(list);

      // Fetch stats for each project in parallel
      const entries = await Promise.all(
        list.map(async (p) => {
          try {
            const stats = await invoke<ProjectStats>("get_project_stats", {
              projectId: p.id,
            });
            return [p.id, stats] as const;
          } catch {
            return [p.id, null] as const;
          }
        })
      );

      const map: Record<string, ProjectStats> = {};
      for (const [id, stats] of entries) {
        if (stats) map[id] = stats;
      }
      setStatsMap(map);
    } catch (err) {
      console.error("Failed to load projects:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  return (
    <div className="p-8 max-w-6xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <h1 className="text-2xl font-bold text-gray-100">Projects</h1>
        <button
          onClick={() => setShowModal(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-xl text-sm font-medium text-white transition-colors"
        >
          + New Project
        </button>
      </div>

      {/* Content */}
      {loading ? (
        <div className="flex items-center justify-center py-20">
          <div className="w-6 h-6 border-2 border-white/20 border-t-white/60 rounded-full animate-spin" />
        </div>
      ) : projects.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <p className="text-gray-500 text-lg mb-4">
            No projects yet. Create one to get started.
          </p>
          <button
            onClick={() => setShowModal(true)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-xl text-sm font-medium text-white transition-colors"
          >
            Create Project
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              stats={statsMap[project.id] ?? null}
              onDeleted={loadProjects}
            />
          ))}
        </div>
      )}

      {/* Modal */}
      {showModal && (
        <NewProjectModal
          onClose={() => {
            setShowModal(false);
            loadProjects();
          }}
        />
      )}
    </div>
  );
}
