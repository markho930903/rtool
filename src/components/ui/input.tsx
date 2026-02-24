import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";

import type { InputVariant, UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type InputSize = Extract<UiSize, "sm" | "default" | "md">;

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
    "w-full rounded-md border border-border-glass bg-surface-glass-soft px-3 py-1.5 text-sm text-text-primary shadow-inset-soft outline-none transition-colors placeholder:text-text-muted focus:border-border-glass-strong",
  tool: "w-full rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-1.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-colors placeholder:text-text-muted focus:border-border-glass-strong",
  clipboard:
    "w-full rounded-md border border-border-glass bg-surface-glass-soft px-2.5 py-1.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-colors placeholder:text-text-muted focus:border-border-glass-strong",
  palette:
    "w-full border-none bg-transparent text-ui-md leading-ui-md text-text-primary outline-none placeholder:text-text-muted",
  theme:
    "h-8 rounded-lg border border-border-glass bg-surface-glass-soft px-2 text-xs text-text-secondary shadow-inset-soft outline-none transition-colors placeholder:text-text-muted focus:border-border-glass-strong",
};

const sizeClassMap: Record<InputSize, string> = {
  sm: "py-1 text-xs",
  default: "",
  md: "py-2 text-sm",
};

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(props, ref) {
  const {
    variant = "default",
    size = "default",
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
