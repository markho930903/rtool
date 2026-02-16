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
    "w-full rounded-md border border-border-muted bg-surface px-3 py-2 text-sm text-text-primary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
  tool:
    "min-h-[110px] w-full rounded-md border border-border-muted bg-surface px-2.5 py-2 text-ui-sm leading-ui-sm text-text-primary outline-none transition-colors placeholder:text-text-muted focus:border-accent",
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
    invalid ? "border-danger focus:border-danger" : null,
    disabled ? "cursor-not-allowed opacity-60" : null,
    className,
  );

  return <textarea {...rest} ref={ref} disabled={disabled} className={textareaClassName} />;
});
