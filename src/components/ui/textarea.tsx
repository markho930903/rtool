import { forwardRef, type TextareaHTMLAttributes } from "react";

import type { TextareaVariant } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type ResizeMode = "none" | "vertical" | "both";

export interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: TextareaVariant;
  resize?: ResizeMode;
  invalid?: boolean;
}

const variantClassMap: Record<TextareaVariant, string> = {
  default:
    "w-full rounded-md border border-border-strong bg-surface-soft px-3 py-2 text-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
  tool: "min-h-[110px] w-full rounded-md border border-border-strong bg-surface-soft px-2.5 py-2 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 placeholder:text-text-muted focus:border-accent focus:ring-2 focus:ring-accent-soft",
};

const resizeClassMap: Record<ResizeMode, string> = {
  none: "resize-none",
  vertical: "resize-y",
  both: "resize",
};

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(function Textarea(props, ref) {
  const { variant = "default", resize = "vertical", invalid = false, className, disabled, ...rest } = props;
  const textareaClassName = cx(
    variantClassMap[variant],
    resizeClassMap[resize],
    invalid ? "border-danger focus:border-danger focus:ring-danger/20" : null,
    disabled ? "cursor-not-allowed opacity-60" : null,
    className,
  );

  return <textarea {...rest} ref={ref} disabled={disabled} className={textareaClassName} />;
});
