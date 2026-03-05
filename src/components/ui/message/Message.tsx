import { memo, type ReactElement } from "react";

import type { MessageRenderProps, MessageType } from "@ui/message/types";
import { cx } from "@/components/ui/utils";

const toneClassMap: Record<MessageType, string> = {
  success: "border-success/35 bg-success/10 text-success",
  info: "border-border-glass bg-surface-glass-soft text-text-secondary",
  warning: "border-warning/35 bg-warning/10 text-warning",
  error: "border-danger/35 bg-danger/10 text-danger",
  loading: "border-accent/35 bg-accent/10 text-accent",
};

const iconClassMap: Record<MessageType, string> = {
  success: "i-lucide:circle-check-big",
  info: "i-lucide:info",
  warning: "i-lucide:triangle-alert",
  error: "i-lucide:circle-alert",
  loading: "i-lucide:loader-circle",
};

const titleClassMap: Record<MessageType, string> = {
  success: "text-success",
  info: "text-text-primary",
  warning: "text-warning",
  error: "text-danger",
  loading: "text-accent",
};

const bodyClassMap: Record<MessageType, string> = {
  success: "text-success",
  info: "text-text-secondary",
  warning: "text-warning",
  error: "text-danger",
  loading: "text-accent",
};

export interface MessageProps extends MessageRenderProps {}

function MessageImpl(props: MessageProps): ReactElement {
  const {
    mode = "inline",
    type = "info",
    title,
    description,
    content,
    icon,
    showIcon = true,
    closable = false,
    closeLabel = "Close",
    className,
    style,
    role,
    actions,
    onClose,
    children,
  } = props;

  const hasBody = description !== undefined || content !== undefined || children !== undefined;
  const bodyNode = description ?? content;
  const computedRole = role ?? (type === "error" ? "alert" : "status");
  const ariaLive = computedRole === "alert" ? "assertive" : "polite";

  return (
    <section
      className={cx(
        "relative rounded-md border px-3 py-2 text-xs",
        "backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
        mode === "toast" ? "pointer-events-auto shadow-overlay" : "shadow-inset-soft",
        toneClassMap[type],
        className,
      )}
      style={style}
      role={computedRole}
      aria-live={ariaLive}
    >
      <div className="flex items-start gap-2.5">
        {showIcon ? (
          <span className={cx("btn-icon mt-0.5 h-4 w-4 shrink-0 text-[1rem]", type === "loading" ? "animate-spin" : null)} aria-hidden="true">
            {icon ?? <span className={iconClassMap[type]} />}
          </span>
        ) : null}

        <div className="min-w-0 flex-1">
          {title !== undefined ? <div className={cx("font-medium", titleClassMap[type])}>{title}</div> : null}
          {hasBody ? (
            <div className={cx(title !== undefined ? "mt-1" : null, "min-w-0 break-words", bodyClassMap[type])}>
              {bodyNode}
              {children !== undefined ? <div className={cx(bodyNode !== undefined ? "mt-1" : null)}>{children}</div> : null}
            </div>
          ) : null}
          {actions !== undefined ? <div className="mt-2 flex flex-wrap items-center gap-2">{actions}</div> : null}
        </div>

        {closable ? (
          <button
            type="button"
            aria-label={closeLabel}
            className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-text-muted transition-colors hover:text-text-primary"
            onClick={onClose}
          >
            <span className="btn-icon i-lucide:x text-[0.95rem]" aria-hidden="true" />
          </button>
        ) : null}
      </div>
    </section>
  );
}

export const Message = memo(MessageImpl);
