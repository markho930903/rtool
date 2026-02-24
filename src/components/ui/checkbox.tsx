import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";

import type { UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type CheckboxSize = Extract<UiSize, "sm" | "default" | "md">;
type CheckboxAlign = "auto" | "center" | "start";

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  size?: CheckboxSize;
  align?: CheckboxAlign;
  label?: ReactNode;
  description?: ReactNode;
  wrapperClassName?: string;
  labelClassName?: string;
}

const sizeClassMap: Record<CheckboxSize, string> = {
  sm: "h-3.5 w-3.5",
  default: "h-4 w-4",
  md: "h-[18px] w-[18px]",
};

const wrapperAlignClassMap: Record<Exclude<CheckboxAlign, "auto">, string> = {
  center: "items-center",
  start: "items-start",
};

const startAlignOffsetClassMap: Record<CheckboxSize, string> = {
  sm: "mt-px",
  default: "mt-0.5",
  md: "mt-0.5",
};

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(function Checkbox(props, ref) {
  const {
    size = "default",
    align = "auto",
    label,
    description,
    wrapperClassName,
    labelClassName,
    className,
    disabled,
    children,
    ...rest
  } = props;
  const finalLabel = label ?? children;
  const hasLabelContent = Boolean(finalLabel || description);
  const resolvedAlign: Exclude<CheckboxAlign, "auto"> = align === "auto" ? (description ? "start" : "center") : align;

  const inputClassName = cx(
    "m-0 shrink-0 rounded-sm border border-border-strong bg-surface text-accent accent-accent",
    "outline-none focus-visible:ring-2 focus-visible:ring-accent/55 focus-visible:ring-offset-1 focus-visible:ring-offset-app",
    sizeClassMap[size],
    hasLabelContent && resolvedAlign === "start" ? startAlignOffsetClassMap[size] : null,
    disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
    className,
  );

  if (!finalLabel && !description) {
    return <input {...rest} ref={ref} type="checkbox" disabled={disabled} className={inputClassName} />;
  }

  return (
    <label
      className={cx(
        "inline-flex gap-2 text-sm text-text-secondary",
        wrapperAlignClassMap[resolvedAlign],
        wrapperClassName,
      )}
    >
      <input {...rest} ref={ref} type="checkbox" disabled={disabled} className={inputClassName} />
      <span className={cx("inline-flex min-w-0 flex-col gap-0.5", labelClassName)}>
        {finalLabel ? <span className="leading-5">{finalLabel}</span> : null}
        {description ? <span className="text-xs leading-5 text-text-muted">{description}</span> : null}
      </span>
    </label>
  );
});
