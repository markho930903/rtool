import { useEffect, type ReactNode } from "react";

import { cx } from "@/components/ui/utils";

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  overlayClassName?: string;
  zIndexClassName?: string;
  closeOnBackdrop?: boolean;
  closeOnEscape?: boolean;
  canClose?: boolean;
  ariaLabel?: string;
  ariaLabelledBy?: string;
}

export function Dialog(props: DialogProps) {
  const {
    open,
    onClose,
    children,
    className,
    overlayClassName,
    zIndexClassName = "z-[70]",
    closeOnBackdrop = true,
    closeOnEscape = true,
    canClose = true,
    ariaLabel,
    ariaLabelledBy,
  } = props;

  useEffect(() => {
    if (!open || !closeOnEscape || !canClose) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      if (typeof event.stopImmediatePropagation === "function") {
        event.stopImmediatePropagation();
      }
      onClose();
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open, closeOnEscape, canClose, onClose]);

  if (!open) {
    return null;
  }

  const handleBackdropClick = () => {
    if (!closeOnBackdrop || !canClose) {
      return;
    }
    onClose();
  };

  return (
    <div className={cx("fixed inset-0", zIndexClassName)} onClick={handleBackdropClick}>
      <div className={cx("absolute inset-0 bg-surface-scrim", overlayClassName)} />
      <section
        className={cx("relative", className)}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </section>
    </div>
  );
}
