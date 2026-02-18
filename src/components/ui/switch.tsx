import { forwardRef, useId, type InputHTMLAttributes, type ReactNode } from "react";

import type { UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type SwitchSize = Extract<UiSize, "sm" | "default" | "md">;
type SwitchControlPosition = "start" | "end";

export interface SwitchProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  size?: SwitchSize;
  invalid?: boolean;
  trackClassName?: string;
  thumbClassName?: string;
}

export interface SwitchFieldProps extends Omit<SwitchProps, "className"> {
  label?: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  wrapperClassName?: string;
  labelClassName?: string;
  switchClassName?: string;
  controlPosition?: SwitchControlPosition;
}

const sizeClassMap: Record<SwitchSize, { root: string; thumb: string; translate: string }> = {
  sm: {
    root: "h-4 w-8",
    thumb: "h-3 w-3",
    translate: "peer-checked:translate-x-4",
  },
  default: {
    root: "h-5 w-9",
    thumb: "h-4 w-4",
    translate: "peer-checked:translate-x-4",
  },
  md: {
    root: "h-6 w-10",
    thumb: "h-5 w-5",
    translate: "peer-checked:translate-x-4",
  },
};

export const Switch = forwardRef<HTMLInputElement, SwitchProps>(function Switch(props, ref) {
  const { size = "default", invalid = false, className, trackClassName, thumbClassName, disabled, id, ...rest } = props;

  const sizeClass = sizeClassMap[size];

  return (
    <span
      className={cx(
        "relative inline-flex shrink-0 align-middle",
        sizeClass.root,
        disabled ? "opacity-70" : null,
        className,
      )}
    >
      <input
        {...rest}
        id={id}
        ref={ref}
        type="checkbox"
        disabled={disabled}
        aria-invalid={invalid || undefined}
        className="peer absolute inset-0 z-10 m-0 h-full w-full cursor-pointer opacity-0 disabled:cursor-not-allowed"
      />
      <span
        aria-hidden="true"
        className={cx(
          "pointer-events-none absolute inset-0 rounded-full bg-surface shadow-inset-soft",
          "border transition-[background-color,border-color,box-shadow] duration-180 ease-out",
          "peer-focus-visible:ring-2 peer-focus-visible:ring-accent/55 peer-focus-visible:ring-offset-1 peer-focus-visible:ring-offset-app",
          "peer-checked:border-accent peer-checked:bg-accent",
          invalid
            ? "border-danger/80 peer-focus-visible:ring-danger/45 peer-checked:border-danger peer-checked:bg-danger"
            : "border-border-strong",
          trackClassName,
        )}
      />
      <span
        aria-hidden="true"
        className={cx(
          "pointer-events-none absolute left-0.5 top-0.5 rounded-full bg-text-primary",
          "shadow-[0_1px_3px_rgb(0_0_0/0.35)] transition-[transform,background-color] duration-180 ease-out",
          sizeClass.thumb,
          sizeClass.translate,
          "peer-checked:bg-accent-contrast",
          thumbClassName,
        )}
      />
    </span>
  );
});

export function SwitchField(props: SwitchFieldProps) {
  const {
    label,
    description,
    children,
    wrapperClassName,
    labelClassName,
    switchClassName,
    controlPosition = "start",
    id,
    disabled,
    ...switchProps
  } = props;
  const generatedId = useId();
  const switchId = id ?? generatedId;
  const finalLabel = label ?? children;
  const hasLabelContent = Boolean(finalLabel || description);

  const controlNode = <Switch {...switchProps} id={switchId} disabled={disabled} className={switchClassName} />;

  if (!hasLabelContent) {
    return controlNode;
  }

  return (
    <div
      className={cx(
        "inline-flex w-full items-start gap-2 text-sm text-text-secondary",
        controlPosition === "end" ? "justify-between" : null,
        wrapperClassName,
      )}
    >
      {controlPosition === "start" ? controlNode : null}
      <label
        htmlFor={switchId}
        className={cx(
          "inline-flex min-w-0 flex-col gap-0.5",
          controlPosition === "end" ? "flex-1" : null,
          disabled ? "cursor-not-allowed opacity-70" : "cursor-pointer",
          labelClassName,
        )}
      >
        {finalLabel ? <span className="leading-5">{finalLabel}</span> : null}
        {description ? <span className="text-xs leading-5 text-text-muted">{description}</span> : null}
      </label>
      {controlPosition === "end" ? controlNode : null}
    </div>
  );
}
