interface PermissionCardProps {
  name: string;
  granted: boolean;
  description: string;
  onOpenSettings: () => void;
}

export function PermissionCard({
  name,
  granted,
  description,
  onOpenSettings,
}: PermissionCardProps) {
  return (
    <div
      className={`p-4 rounded-lg border-2 ${
        granted
          ? "border-statusSuccess bg-statusSuccess/10"
          : "border-error bg-error/10"
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-lg">{granted ? "✓" : "✗"}</span>
          <span className="text-[14px] font-medium text-textAlt lowercase">
            {name}
          </span>
        </div>
        {granted ? (
          <span className="text-[11px] text-statusSuccess lowercase font-mono">
            granted
          </span>
        ) : (
          <span className="text-[11px] text-error lowercase font-mono">
            required
          </span>
        )}
      </div>
      {!granted && (
        <>
          <p className="text-[12px] text-textSecondary lowercase mb-3">
            {description}
          </p>
          <button
            onClick={onOpenSettings}
            className="text-[12px] lowercase px-3 py-1.5 rounded bg-error hover:bg-errorLight text-black font-medium transition-colors"
          >
            open settings
          </button>
        </>
      )}
    </div>
  );
}
