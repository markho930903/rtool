import { cx } from "@/components/ui/utils";
import type { ReactNode } from "react";

export type LoadingIndicatorSize = "xs" | "sm";
export type LoadingIndicatorMode = "inline" | "overlay";

export interface LoadingIndicatorProps {
  loading?: boolean;
  mode?: LoadingIndicatorMode;
  text?: string;
  size?: LoadingIndicatorSize;
  className?: string;
  iconClassName?: string;
  textClassName?: string;
  ariaLabel?: string;
  children?: ReactNode;
  containerClassName?: string;
  overlayClassName?: string;
  maskClassName?: string;
  minHeightClassName?: string;
  blockInteraction?: boolean;
  showMask?: boolean;
}

interface SizeConfig {
  iconClassName: string;
  textClassName: string;
}

interface IndicatorViewProps {
  ariaLabel: string;
  className?: string;
  iconClassName?: string;
  size: LoadingIndicatorSize;
  text?: string;
  textClassName?: string;
}

const SIZE_CONFIG_MAP: Record<LoadingIndicatorSize, SizeConfig> = {
  xs: {
    iconClassName: "text-[12px]",
    textClassName: "text-xs",
  },
  sm: {
    iconClassName: "text-sm",
    textClassName: "text-sm",
  },
};

function IndicatorView(props: IndicatorViewProps) {
  const sizeConfig = SIZE_CONFIG_MAP[props.size];
  return (
    <span className={cx("inline-flex items-center gap-1 text-text-muted", props.className)}>
      <span
        className={cx("i-noto:hourglass-not-done animate-spin", sizeConfig.iconClassName, props.iconClassName)}
        aria-label={props.text ? undefined : props.ariaLabel}
        aria-hidden={props.text ? "true" : undefined}
        role={props.text ? undefined : "img"}
      />
      {props.text ? <span className={cx(sizeConfig.textClassName, props.textClassName)}>{props.text}</span> : null}
    </span>
  );
}

export default function LoadingIndicator(props: LoadingIndicatorProps) {
  const loading = props.loading ?? true;
  const mode = props.mode ?? "inline";
  const size = props.size ?? "xs";
  const showMask = props.showMask ?? true;
  const blockInteraction = props.blockInteraction ?? true;
  const minHeightClassName = props.minHeightClassName ?? "min-h-24";
  const ariaLabel = props.ariaLabel ?? props.text ?? "Loading";
  const hasChildren = props.children !== null && props.children !== undefined;

  if (mode === "inline") {
    if (!loading) {
      return null;
    }

    return (
      <IndicatorView
        ariaLabel={ariaLabel}
        className={props.className}
        iconClassName={props.iconClassName}
        size={size}
        text={props.text}
        textClassName={props.textClassName}
      />
    );
  }

  if (!loading && !hasChildren) {
    return null;
  }

  return (
    <div className={cx("relative", hasChildren ? undefined : minHeightClassName, props.containerClassName)}>
      {props.children}
      {loading ? (
        <div
          className={cx(
            "absolute inset-0 z-[1] flex items-center justify-center",
            blockInteraction ? "pointer-events-auto" : "pointer-events-none",
            props.overlayClassName
          )}
          aria-label={ariaLabel}
          aria-live="polite"
          role="status"
        >
          {showMask ? <div aria-hidden="true" className={cx("absolute inset-0 bg-surface-scrim/26", props.maskClassName)} /> : null}
          <IndicatorView
            ariaLabel={ariaLabel}
            className={cx("relative", props.className)}
            iconClassName={props.iconClassName}
            size={size}
            text={props.text}
            textClassName={props.textClassName}
          />
        </div>
      ) : null}
    </div>
  );
}
