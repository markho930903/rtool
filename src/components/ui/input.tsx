import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";

import type { InputVariant } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type InputSize = "sm" | "md";

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  variant?: InputVariant;
  size?: InputSize;
  invalid?: boolean;
  leadingIcon?: ReactNode;
  trailingSlot?: ReactNode;
  wrapperClassName?: string;
}

const variantClassMap: Record<InputVariant, string> = {
  default:
    "w-full rounded-md border border-border-muted bg-surface px-3 py-2 text-sm text-text-primary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
  tool: "w-full rounded-md border border-border-muted bg-surface px-2.5 py-2 text-[13px] text-text-primary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
  clipboard:
    "w-full rounded-md border border-border-muted bg-surface px-2.5 py-2 text-[13px] text-text-primary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
  palette: "w-full border-none bg-transparent text-[15px] text-text-primary outline-none placeholder:text-text-muted",
  theme:
    "h-8 rounded-lg border border-border-muted bg-surface px-2 text-xs text-text-secondary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
};

const sizeClassMap: Record<InputSize, string> = {
  sm: "text-xs",
  md: "",
};

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(props, ref) {
  const {
    variant = "default",
    size = "md",
    invalid = false,
    leadingIcon,
    trailingSlot,
    wrapperClassName,
    className,
    disabled,
    ...rest
  } = props;

  const inputClassName = cx(
    variantClassMap[variant],
    sizeClassMap[size],
    invalid ? "border-danger focus:border-danger" : null,
    disabled ? "cursor-not-allowed opacity-60" : null,
    className,
  );

  const hasWrapper = Boolean(leadingIcon || trailingSlot || wrapperClassName);
  if (!hasWrapper) {
    return <input {...rest} ref={ref} disabled={disabled} className={inputClassName} />;
  }

  return (
    <div className={cx("flex items-center gap-2", wrapperClassName)}>
      {leadingIcon ? <span className="shrink-0">{leadingIcon}</span> : null}
      <input {...rest} ref={ref} disabled={disabled} className={inputClassName} />
      {trailingSlot ? <span className="shrink-0">{trailingSlot}</span> : null}
    </div>
  );
});
