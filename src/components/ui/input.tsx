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
    "w-full rounded-md border border-border-strong bg-surface-soft px-3 py-1.5 text-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
  tool: "w-full rounded-md border border-border-strong bg-surface-soft px-2.5 py-1.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
  clipboard:
    "w-full rounded-md border border-border-strong bg-surface-soft px-2.5 py-1.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
  palette:
    "w-full border-none bg-transparent text-ui-md leading-ui-md text-text-primary outline-none placeholder:text-text-muted",
  theme:
    "h-8 rounded-md border border-border-strong bg-surface-soft px-2 text-xs text-text-secondary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
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
    invalid ? "border-danger focus:border-danger focus:ring-danger/20" : null,
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
