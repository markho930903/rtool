import {
  forwardRef,
  type ForwardedRef,
  type AnchorHTMLAttributes,
  type ButtonHTMLAttributes,
  type MouseEvent,
  type ReactNode,
} from "react";
import { Link, type LinkProps } from "react-router";

import type { UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

interface BaseButtonProps {
  children: ReactNode;
  className?: string;
  variant?: ButtonVariant;
  size?: UiSize;
  block?: boolean;
  iconOnly?: boolean;
  disabled?: boolean;
  unstyled?: boolean;
}

type NativeButtonProps = BaseButtonProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children" | "disabled" | "size"> & {
    as?: "button";
  };

type AnchorButtonProps = BaseButtonProps &
  Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "className" | "children"> & {
    as: "a";
  };

type LinkButtonProps = BaseButtonProps & Omit<LinkProps, "className" | "children"> & { as: "link" };

export type ButtonProps = NativeButtonProps | AnchorButtonProps | LinkButtonProps;

const sizeClassMap: Record<UiSize, string> = {
  xs: "rounded-sm px-2 py-1 text-xs",
  sm: "rounded-sm px-2.5 py-1.5 text-xs",
  default: "rounded-md px-3 py-1.5 text-sm",
  md: "rounded-lg px-3.5 py-2 text-sm font-medium",
  lg: "rounded-xl px-4 py-2.5 text-base font-medium",
};

const iconOnlySizeClassMap: Record<UiSize, string> = {
  xs: "h-7 w-7",
  sm: "h-8 w-8",
  default: "h-9 w-9",
  md: "h-10 w-10",
  lg: "h-11 w-11",
};

const variantClassMap: Record<ButtonVariant, string> = {
  primary: "border-transparent bg-accent text-accent-contrast hover:opacity-90",
  secondary:
    "border-border-glass bg-surface-glass-soft text-text-primary shadow-inset-soft hover:border-border-glass-strong hover:bg-surface-glass",
  danger:
    "border-border-glass bg-surface-glass-soft text-danger shadow-inset-soft hover:border-danger hover:bg-surface-glass",
  ghost: "border-transparent bg-transparent text-text-secondary hover:bg-surface-glass-soft hover:text-text-primary",
};

function buildClassName(props: {
  className?: string;
  variant: ButtonVariant;
  size: UiSize;
  block: boolean;
  iconOnly: boolean;
  disabled: boolean;
  unstyled: boolean;
}) {
  if (props.unstyled) {
    return cx(props.className, props.disabled ? "cursor-not-allowed opacity-60 pointer-events-none" : null);
  }

  return cx(
    "inline-flex items-center justify-center gap-1.5 border no-underline transition-colors duration-[140ms]",
    "outline-none focus-visible:ring-2 focus-visible:ring-accent/55 focus-visible:ring-offset-1 focus-visible:ring-offset-app",
    sizeClassMap[props.size],
    variantClassMap[props.variant],
    props.iconOnly ? cx(iconOnlySizeClassMap[props.size], "px-0 py-0") : null,
    props.block ? "w-full" : null,
    props.disabled ? "cursor-not-allowed opacity-60 pointer-events-none" : "cursor-pointer",
    props.className,
  );
}

export const Button = forwardRef<HTMLButtonElement | HTMLAnchorElement, ButtonProps>(function Button(props, ref) {
  const variant = props.variant ?? "secondary";
  const size = props.size ?? "default";
  const disabled = props.disabled ?? false;
  const block = props.block ?? false;
  const iconOnly = props.iconOnly ?? false;
  const unstyled = props.unstyled ?? false;
  const className = buildClassName({ className: props.className, variant, size, block, iconOnly, disabled, unstyled });

  if (props.as === "link") {
    const { as: linkTag, onClick, tabIndex, ...linkProps } = props;
    void linkTag;

    return (
      <Link
        {...linkProps}
        ref={ref as ForwardedRef<HTMLAnchorElement>}
        tabIndex={disabled ? -1 : tabIndex}
        aria-disabled={disabled || undefined}
        className={className}
        onClick={(event: MouseEvent<HTMLAnchorElement>) => {
          if (disabled) {
            event.preventDefault();
            event.stopPropagation();
            return;
          }

          onClick?.(event);
        }}
      />
    );
  }

  if (props.as === "a") {
    const { as: anchorTag, onClick, tabIndex, ...anchorProps } = props;
    void anchorTag;

    return (
      <a
        {...anchorProps}
        ref={ref as ForwardedRef<HTMLAnchorElement>}
        tabIndex={disabled ? -1 : tabIndex}
        aria-disabled={disabled || undefined}
        className={className}
        onClick={(event: MouseEvent<HTMLAnchorElement>) => {
          if (disabled) {
            event.preventDefault();
            event.stopPropagation();
            return;
          }

          onClick?.(event);
        }}
      />
    );
  }

  const { as: buttonTag, type, ...buttonProps } = props;
  void buttonTag;
  return (
    <button
      {...buttonProps}
      ref={ref as ForwardedRef<HTMLButtonElement>}
      type={type ?? "button"}
      disabled={disabled}
      className={className}
    />
  );
});
