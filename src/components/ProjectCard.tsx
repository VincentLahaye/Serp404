import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";

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

interface Project {
  id: string;
  domain: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

interface ProjectCardProps {
  project: Project;
  stats: ProjectStats | null;
  onDeleted: () => void;
}

function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;

  if (isNaN(then)) return dateStr;

  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return "just now";

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;

  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} day${days === 1 ? "" : "s"} ago`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months} month${months === 1 ? "" : "s"} ago`;

  const years = Math.floor(months / 12);
  return `${years} year${years === 1 ? "" : "s"} ago`;
}

function statusBadge(status: string) {
  switch (status.toLowerCase()) {
    case "running":
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-500/15 text-blue-400">
          <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
          Running
        </span>
      );
    case "done":
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-500/15 text-green-400">
          <span className="w-1.5 h-1.5 rounded-full bg-green-400" />
          Done
        </span>
      );
    default:
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-500/15 text-gray-400">
          <span className="w-1.5 h-1.5 rounded-full bg-gray-400" />
          Created
        </span>
      );
  }
}

export default function ProjectCard({ project, stats, onDeleted }: ProjectCardProps) {
  const navigate = useNavigate();

  function handleDelete(e: React.MouseEvent) {
    e.stopPropagation();
    if (!window.confirm(`Delete project "${project.domain}"? This cannot be undone.`)) return;

    invoke("delete_project", { id: project.id }).then(() => {
      onDeleted();
    });
  }

  return (
    <div
      onClick={() => navigate(`/project/${project.id}`)}
      className="relative bg-white/5 border border-white/10 rounded-xl p-5 hover:bg-white/[0.08] hover:border-white/15 transition-all cursor-pointer group"
    >
      {/* Delete button */}
      <button
        onClick={handleDelete}
        className="absolute top-3 right-3 p-1.5 rounded-lg text-gray-500 hover:text-red-400 hover:bg-red-400/10 transition-colors opacity-0 group-hover:opacity-100"
        aria-label="Delete project"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M3 6h18" />
          <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
          <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
          <line x1="10" y1="11" x2="10" y2="17" />
          <line x1="14" y1="11" x2="14" y2="17" />
        </svg>
      </button>

      {/* Domain */}
      <h3 className="text-lg font-semibold text-gray-100 truncate pr-8">
        {project.domain}
      </h3>

      {/* Created date */}
      <p className="text-sm text-gray-500 mt-1">
        {relativeTime(project.createdAt)}
      </p>

      {/* Status badge */}
      <div className="mt-3">
        {statusBadge(project.status)}
      </div>

      {/* Stats line */}
      {stats && (
        <p className="text-xs text-gray-500 mt-3">
          {stats.totalUrls} URL{stats.totalUrls !== 1 ? "s" : ""}
          {" | "}
          {stats.errorCount} error{stats.errorCount !== 1 ? "s" : ""}
        </p>
      )}
    </div>
  );
}
