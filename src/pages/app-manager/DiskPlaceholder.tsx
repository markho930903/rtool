export function DiskPlaceholder({ title, desc }: { title: string; desc: string }) {
  return (
    <div className="ui-glass-panel flex h-full min-h-[320px] flex-col items-center justify-center gap-4 px-6 py-12 text-center">
      <svg width="110" height="110" viewBox="0 0 110 110" fill="none" aria-hidden="true">
        <circle cx="55" cy="55" r="47" stroke="currentColor" className="text-border-glass-strong" strokeWidth="4" />
        <circle cx="55" cy="55" r="32" stroke="currentColor" className="text-text-muted" strokeWidth="4" />
        <circle cx="55" cy="55" r="8" fill="currentColor" className="text-text-secondary" />
        <path
          d="M55 8C80.957 8 102 29.043 102 55"
          stroke="currentColor"
          className="text-accent"
          strokeWidth="4"
          strokeLinecap="round"
        />
      </svg>
      <div className="space-y-1">
        <h2 className="m-0 text-lg font-semibold text-text-primary">{title}</h2>
        <p className="m-0 text-sm text-text-secondary">{desc}</p>
      </div>
    </div>
  );
}
