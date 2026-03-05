interface AuditStats {
  okCount: number;
  redirectCount: number;
  notFoundCount: number;
  errorCount: number;
  emptyTitleCount: number;
  slowCount: number;
}

interface StatsBarProps {
  stats: AuditStats;
}

interface StatCard {
  label: string;
  value: number;
  dotColor: string;
}

export default function StatsBar({ stats }: StatsBarProps) {
  const cards: StatCard[] = [
    { label: "OK", value: stats.okCount, dotColor: "bg-green-400" },
    { label: "Redirects", value: stats.redirectCount, dotColor: "bg-yellow-400" },
    { label: "404", value: stats.notFoundCount, dotColor: "bg-red-400" },
    { label: "Errors", value: stats.errorCount, dotColor: "bg-red-500" },
    { label: "Empty Titles", value: stats.emptyTitleCount, dotColor: "bg-orange-400" },
    { label: "Slow (>2s)", value: stats.slowCount, dotColor: "bg-orange-400" },
  ];

  return (
    <div className="grid grid-cols-3 gap-3 sm:grid-cols-6">
      {cards.map((card) => (
        <div
          key={card.label}
          className="bg-white/5 rounded-lg p-3 border border-white/10 flex flex-col items-center gap-1"
        >
          <div className="flex items-center gap-1.5">
            <span
              className={`w-2 h-2 rounded-full ${card.dotColor} inline-block`}
            />
            <span className="text-xs text-gray-400">{card.label}</span>
          </div>
          <span className="text-lg font-semibold text-gray-100">
            {card.value}
          </span>
        </div>
      ))}
    </div>
  );
}

export type { AuditStats };
