import { useId, type ReactNode } from "react";

import { cx } from "@/components/ui/utils";

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  className?: string;
  triggerClassName?: string;
  panelClassName?: string;
  ariaLabel?: string;
}

export function Tooltip(props: TooltipProps) {
  const { content, children, className, triggerClassName, panelClassName, ariaLabel } = props;
  const tooltipId = useId();

  return (
    <span className={cx("group relative inline-flex", className)}>
      <span
        tabIndex={0}
        aria-label={ariaLabel}
        aria-describedby={tooltipId}
        className={cx(
          "inline-flex items-center outline-none",
          "focus-visible:ring-2 focus-visible:ring-accent/55 focus-visible:ring-offset-1 focus-visible:ring-offset-app",
          triggerClassName,
        )}
      >
        {children}
      </span>
      <span
        id={tooltipId}
        role="tooltip"
        className={cx(
          "pointer-events-none absolute left-1/2 top-0 z-20 w-max max-w-72",
          "-translate-x-1/2 -translate-y-[calc(100%+8px)]",
          "rounded-md border border-border-muted bg-surface-card px-2 py-1",
          "text-xs leading-5 text-text-secondary shadow-surface",
          "whitespace-normal break-words opacity-0 transition-[opacity,transform] duration-150 ease-out",
          "group-hover:opacity-100 group-hover:-translate-y-[calc(100%+10px)]",
          "group-focus-within:opacity-100 group-focus-within:-translate-y-[calc(100%+10px)]",
          panelClassName,
        )}
      >
        {content}
      </span>
    </span>
  );
}
