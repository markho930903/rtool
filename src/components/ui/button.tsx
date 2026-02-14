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
  xs: "rounded-sm px-2.5 py-1.5 text-xs",
  sm: "rounded-md px-3 py-1.5 text-sm",
  md: "rounded-xl px-4 py-2 text-sm font-medium",
};

const variantClassMap: Record<ButtonVariant, string> = {
  primary: "border-transparent bg-accent text-accent-contrast hover:opacity-90",
  secondary: "border-border-strong bg-surface text-text-primary hover:border-accent hover:bg-surface-soft",
  danger: "border-border-strong bg-surface text-danger hover:border-danger hover:bg-surface-soft",
  ghost: "border-transparent bg-transparent text-text-secondary hover:bg-surface-soft hover:text-text-primary",
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
    props.iconOnly ? "h-8 w-8 px-0 py-0" : null,
    props.block ? "w-full" : null,
    props.disabled ? "cursor-not-allowed opacity-60 pointer-events-none" : "cursor-pointer",
    props.className,
  );
}

export const Button = forwardRef<HTMLButtonElement | HTMLAnchorElement, ButtonProps>(function Button(props, ref) {
  const variant = props.variant ?? "secondary";
  const size = props.size ?? "md";
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
