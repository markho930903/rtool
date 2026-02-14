import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";

import { cx } from "@/components/ui/utils";

type CheckboxSize = "sm" | "md";

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  size?: CheckboxSize;
  label?: ReactNode;
  description?: ReactNode;
  wrapperClassName?: string;
  labelClassName?: string;
}

const sizeClassMap: Record<CheckboxSize, string> = {
  sm: "h-3.5 w-3.5",
  md: "h-4 w-4",
};

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(function Checkbox(props, ref) {
  const {
    size = "md",
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

  const inputClassName = cx(
    "m-0 shrink-0 rounded-sm border border-border-strong bg-surface text-accent accent-accent",
    "outline-none focus-visible:ring-2 focus-visible:ring-accent/55 focus-visible:ring-offset-1 focus-visible:ring-offset-app",
    sizeClassMap[size],
    disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
    className,
  );

  if (!finalLabel && !description) {
    return <input {...rest} ref={ref} type="checkbox" disabled={disabled} className={inputClassName} />;
  }

  return (
    <label className={cx("inline-flex items-start gap-2 text-text-secondary", wrapperClassName)}>
      <input {...rest} ref={ref} type="checkbox" disabled={disabled} className={inputClassName} />
      <span className={cx("inline-flex flex-col gap-0.5", labelClassName)}>
        {finalLabel ? <span>{finalLabel}</span> : null}
        {description ? <span className="text-xs text-text-muted">{description}</span> : null}
      </span>
    </label>
  );
});
