import {
  Children,
  type CSSProperties,
  forwardRef,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
  type SelectHTMLAttributes,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { SelectVariant, UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type SelectSize = Extract<UiSize, "sm" | "default" | "md">;

export interface SelectOptionInput {
  value: string;
  label: string;
  icon?: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  variant?: SelectVariant;
  size?: SelectSize;
  invalid?: boolean;
  options?: SelectOptionInput[];
}

interface SelectOption {
  key: string;
  value: string;
  label: string;
  icon?: string;
  disabled: boolean;
}

const wrapperClassMap: Record<SelectVariant, string> = {
  default: "w-full",
  tool: "inline-block",
  clipboard: "inline-block",
  theme: "inline-block",
};

const triggerClassMap: Record<SelectVariant, string> = {
  default:
    "w-full rounded-md border border-border-glass bg-surface-glass-soft px-3 text-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-glass-strong hover:bg-surface-glass",
  tool: "rounded-md border border-border-glass bg-surface-glass-soft px-2.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-glass-strong hover:bg-surface-glass",
  clipboard:
    "rounded-md border border-border-glass bg-surface-glass-soft px-2.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-glass-strong hover:bg-surface-glass",
  theme:
    "h-8 rounded-lg border border-border-glass bg-surface-glass-soft px-2 text-xs text-text-secondary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-glass-strong hover:bg-surface-glass",
};

const panelClassMap: Record<SelectVariant, string> = {
  default:
    "overflow-y-auto rounded-lg border border-border-glass bg-surface-glass-strong p-1 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
  tool: "overflow-y-auto rounded-lg border border-border-glass bg-surface-glass-strong p-1 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
  clipboard:
    "overflow-y-auto rounded-lg border border-border-glass bg-surface-glass-strong p-1 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
  theme:
    "overflow-y-auto rounded-lg border border-border-glass bg-surface-glass-strong p-1 shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
};

const optionClassMap: Record<SelectVariant, string> = {
  default: "min-h-[34px] rounded-md px-2.5 text-sm",
  tool: "min-h-[32px] rounded-md px-2.5 text-ui-sm leading-ui-sm",
  clipboard: "min-h-[32px] rounded-md px-2.5 text-ui-sm leading-ui-sm",
  theme: "min-h-[30px] rounded-md px-2 text-xs",
};

const sizeClassMap: Record<SelectSize, string> = {
  sm: "min-h-[30px] text-xs",
  default: "min-h-[34px] text-sm",
  md: "min-h-[38px] text-sm",
};

const PANEL_GAP = 6;
const VIEWPORT_PADDING = 8;
const PANEL_MAX_HEIGHT = 256;
const MIN_PANEL_HEIGHT = 120;

function clamp(value: number, min: number, max: number): number {
  if (max < min) {
    return min;
  }
  return Math.min(Math.max(value, min), max);
}

function normalizeSelectValue(value: SelectHTMLAttributes<HTMLSelectElement>["value"]): string {
  if (Array.isArray(value)) {
    return value[0] ? String(value[0]) : "";
  }

  if (value === undefined || value === null) {
    return "";
  }

  return String(value);
}

function extractText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }

  if (Array.isArray(node)) {
    return node.map(extractText).join("");
  }

  if (isValidElement<{ children?: ReactNode }>(node)) {
    return extractText(node.props.children);
  }

  return "";
}

function parseOptions(children: ReactNode): SelectOption[] {
  return Children.toArray(children).flatMap((child, index) => {
    if (
      !isValidElement<{ value?: string | number; disabled?: boolean; children?: ReactNode }>(child) ||
      child.type !== "option"
    ) {
      return [];
    }

    const optionValue = child.props.value ?? extractText(child.props.children);
    const value = String(optionValue ?? "");
    const label = extractText(child.props.children) || value;
    const key = child.key ? String(child.key) : `${value}-${index}`;

    return [
      {
        key,
        value,
        label,
        icon: undefined,
        disabled: Boolean(child.props.disabled),
      },
    ];
  });
}

function normalizeOptions(options: SelectOptionInput[]): SelectOption[] {
  return options.map((option, index) => ({
    key: `${option.value}-${index}`,
    value: option.value,
    label: option.label,
    icon: option.icon,
    disabled: Boolean(option.disabled),
  }));
}

function renderNativeOptions(options: SelectOption[]): ReactNode {
  return options.map((option) => (
    <option key={option.key} value={option.value} disabled={option.disabled}>
      {option.label}
    </option>
  ));
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(props, ref) {
  const { t } = useTranslation("common");
  const {
    variant = "default",
    size = "default",
    invalid = false,
    className,
    disabled,
    children,
    options: optionsProp,
    value,
    defaultValue,
    onChange,
    multiple,
    ...rest
  } = props;

  const hasOptionsProp = optionsProp !== undefined;
  const options = useMemo(
    () => (hasOptionsProp ? normalizeOptions(optionsProp ?? []) : parseOptions(children)),
    [children, hasOptionsProp, optionsProp],
  );
  const nativeOptionNodes = useMemo(
    () => (hasOptionsProp ? renderNativeOptions(options) : children),
    [children, hasOptionsProp, options],
  );
  const firstEnabledValue = useMemo(
    () => options.find((option) => !option.disabled)?.value ?? options[0]?.value ?? "",
    [options],
  );

  const controlledValue = value === undefined ? null : normalizeSelectValue(value);
  const [uncontrolledValue, setUncontrolledValue] = useState(
    () => normalizeSelectValue(defaultValue) || firstEnabledValue,
  );
  const selectedValue = controlledValue ?? uncontrolledValue;

  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState<number>(-1);
  const [panelStyle, setPanelStyle] = useState<CSSProperties | null>(null);

  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLUListElement>(null);
  const hiddenSelectRef = useRef<HTMLSelectElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLLIElement | null>>([]);
  const listboxId = useId();

  const selectedOption = options.find((option) => option.value === selectedValue) ?? options[0] ?? null;

  const updatePanelPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger || typeof window === "undefined") {
      return;
    }

    const rect = trigger.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    const spaceBelow = viewportHeight - rect.bottom - VIEWPORT_PADDING;
    const spaceAbove = rect.top - VIEWPORT_PADDING;
    const placeTop = spaceBelow < MIN_PANEL_HEIGHT && spaceAbove > spaceBelow;
    const availableHeight = Math.max(0, (placeTop ? spaceAbove : spaceBelow) - PANEL_GAP);
    const maxHeight = clamp(availableHeight, Math.min(MIN_PANEL_HEIGHT, PANEL_MAX_HEIGHT), PANEL_MAX_HEIGHT);
    const maxWidth = Math.max(160, viewportWidth - VIEWPORT_PADDING * 2);
    const minWidth = Math.min(rect.width, maxWidth);

    const nextStyle: CSSProperties = {
      position: "fixed",
      zIndex: 90,
      maxHeight: `${maxHeight}px`,
      maxWidth: `${maxWidth}px`,
    };

    if (placeTop) {
      nextStyle.bottom = `${viewportHeight - rect.top + PANEL_GAP}px`;
    } else {
      nextStyle.top = `${rect.bottom + PANEL_GAP}px`;
    }

    if (variant === "default") {
      const width = Math.min(rect.width, maxWidth);
      nextStyle.width = `${width}px`;
      nextStyle.left = `${clamp(rect.left, VIEWPORT_PADDING, viewportWidth - VIEWPORT_PADDING - width)}px`;
    } else if (variant === "theme") {
      nextStyle.minWidth = `${minWidth}px`;
      nextStyle.right = `${clamp(
        viewportWidth - rect.right,
        VIEWPORT_PADDING,
        viewportWidth - VIEWPORT_PADDING - minWidth,
      )}px`;
    } else {
      nextStyle.minWidth = `${minWidth}px`;
      nextStyle.left = `${clamp(rect.left, VIEWPORT_PADDING, viewportWidth - VIEWPORT_PADDING - minWidth)}px`;
    }

    setPanelStyle(nextStyle);
  }, [variant]);

  useEffect(() => {
    if (controlledValue !== null) {
      return;
    }

    setUncontrolledValue((previous) => {
      if (options.length === 0) {
        return "";
      }

      const hasPrevious = options.some((option) => option.value === previous);
      return hasPrevious ? previous : firstEnabledValue;
    });
  }, [controlledValue, firstEnabledValue, options]);

  useEffect(() => {
    if (!hiddenSelectRef.current) {
      return;
    }

    hiddenSelectRef.current.value = selectedValue;
  }, [selectedValue]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const selectedIndex = selectedOption ? options.findIndex((option) => option.value === selectedOption.value) : -1;
    const fallbackIndex = options.findIndex((option) => !option.disabled);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : fallbackIndex);
  }, [open, options, selectedOption]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!rootRef.current?.contains(target) && !panelRef.current?.contains(target)) {
        setOpen(false);
      }
    };

    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  useLayoutEffect(() => {
    if (!open) {
      setPanelStyle(null);
      return;
    }

    updatePanelPosition();

    const syncPosition = () => {
      updatePanelPosition();
    };
    window.addEventListener("resize", syncPosition);
    window.addEventListener("scroll", syncPosition, true);

    return () => {
      window.removeEventListener("resize", syncPosition);
      window.removeEventListener("scroll", syncPosition, true);
    };
  }, [open, updatePanelPosition]);

  useEffect(() => {
    if (!open || activeIndex < 0) {
      return;
    }

    optionRefs.current[activeIndex]?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, open]);

  const commitSelection = (nextValue: string) => {
    if (disabled) {
      return;
    }

    if (controlledValue === null) {
      setUncontrolledValue(nextValue);
    }

    if (hiddenSelectRef.current) {
      hiddenSelectRef.current.value = nextValue;
    }

    if (onChange) {
      const target = hiddenSelectRef.current;
      const fallbackTarget = { value: nextValue } as HTMLSelectElement;
      onChange({
        target: target ?? fallbackTarget,
        currentTarget: target ?? fallbackTarget,
      } as React.ChangeEvent<HTMLSelectElement>);
    }

    setOpen(false);
    triggerRef.current?.focus();
  };

  const moveActive = (direction: 1 | -1) => {
    if (options.length === 0) {
      return;
    }

    let current = activeIndex;
    if (current < 0 || current >= options.length) {
      current = options.findIndex((option) => !option.disabled);
    }

    for (let step = 0; step < options.length; step += 1) {
      const next = (current + direction + options.length) % options.length;
      current = next;
      if (!options[next]?.disabled) {
        setActiveIndex(next);
        return;
      }
    }
  };

  const onTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) {
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) {
        setOpen(true);
      } else {
        moveActive(1);
      }
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) {
        setOpen(true);
      } else {
        moveActive(-1);
      }
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (!open) {
        setOpen(true);
        return;
      }

      if (activeIndex >= 0) {
        const activeOption = options[activeIndex];
        if (activeOption && !activeOption.disabled) {
          commitSelection(activeOption.value);
        }
      }
      return;
    }

    if (event.key === "Escape") {
      if (open) {
        event.preventDefault();
        setOpen(false);
      }
      return;
    }

    if (event.key === "Tab") {
      setOpen(false);
    }
  };

  const effectiveSizeClassName = variant === "theme" ? null : sizeClassMap[size];

  if (multiple) {
    const selectClassName = cx(
      triggerClassMap[variant],
      effectiveSizeClassName,
      invalid ? "border-danger focus-visible:border-danger focus-visible:ring-danger/25" : null,
      disabled ? "cursor-not-allowed opacity-60" : null,
      className,
    );

    return (
      <select
        {...rest}
        ref={ref}
        disabled={disabled}
        className={selectClassName}
        value={value}
        defaultValue={defaultValue}
      >
        {nativeOptionNodes}
      </select>
    );
  }

  const triggerClassName = cx(
    "inline-flex w-full items-center justify-between gap-2",
    "focus-visible:border-accent focus-visible:ring-3 focus-visible:ring-accent/25",
    triggerClassMap[variant],
    effectiveSizeClassName,
    invalid ? "border-danger focus-visible:border-danger focus-visible:ring-danger/25" : null,
    disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
    className,
  );

  const panelClassName = cx("fixed z-[90]", panelClassMap[variant]);
  const panelNode =
    open && typeof document !== "undefined"
      ? createPortal(
          <ul
            id={listboxId}
            ref={panelRef}
            role="listbox"
            className={panelClassName}
            style={panelStyle ?? { visibility: "hidden" }}
            aria-activedescendant={activeIndex >= 0 ? `${listboxId}-${activeIndex}` : undefined}
          >
            {options.map((option, index) => {
              const isSelected = selectedValue === option.value;
              const isActive = activeIndex === index;

              return (
                <li
                  id={`${listboxId}-${index}`}
                  key={option.key}
                  ref={(node) => {
                    optionRefs.current[index] = node;
                  }}
                  role="option"
                  aria-selected={isSelected}
                  className={cx(
                    "flex items-center justify-between gap-2",
                    "transition-colors duration-120",
                    optionClassMap[variant],
                    option.disabled
                      ? "cursor-not-allowed text-text-muted/50"
                      : "cursor-pointer text-text-secondary hover:bg-surface-glass-soft hover:text-text-primary",
                    isActive && !option.disabled ? "bg-surface-glass-soft text-text-primary" : null,
                    isSelected && !option.disabled ? "bg-accent-soft text-text-primary" : null,
                  )}
                  onMouseEnter={() => {
                    if (!option.disabled) {
                      setActiveIndex(index);
                    }
                  }}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => {
                    if (!option.disabled) {
                      commitSelection(option.value);
                    }
                  }}
                >
                  <span className="inline-flex min-w-0 items-center gap-2">
                    {option.icon ? <span className={cx("btn-icon shrink-0", option.icon)} aria-hidden="true" /> : null}
                    <span className="truncate">{option.label}</span>
                  </span>
                  {isSelected ? (
                    <svg viewBox="0 0 16 16" className="h-3.5 w-3.5 shrink-0 text-accent" aria-hidden="true">
                      <path
                        d="m3.5 8.25 2.5 2.5L12.5 4.5"
                        fill="none"
                        stroke="currentColor"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth="1.6"
                      />
                    </svg>
                  ) : null}
                </li>
              );
            })}
          </ul>,
          document.body,
        )
      : null;

  return (
    <div ref={rootRef} className={cx("relative", wrapperClassMap[variant])}>
      <select
        {...rest}
        ref={(node) => {
          hiddenSelectRef.current = node;
          if (typeof ref === "function") {
            ref(node);
            return;
          }

          if (ref) {
            ref.current = node;
          }
        }}
        disabled={disabled}
        tabIndex={-1}
        aria-hidden="true"
        className="pointer-events-none absolute h-0 w-0 opacity-0"
        value={selectedValue}
        onChange={onChange}
      >
        {nativeOptionNodes}
      </select>

      <Button
        unstyled
        type="button"
        ref={triggerRef}
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        className={triggerClassName}
        onClick={() => setOpen((previous) => !previous)}
        onKeyDown={onTriggerKeyDown}
      >
        <span className="inline-flex min-w-0 items-center gap-2">
          {selectedOption?.icon ? (
            <span className={cx("btn-icon shrink-0 text-text-muted", selectedOption.icon)} aria-hidden="true" />
          ) : null}
          <span className="truncate text-left">{selectedOption?.label ?? t("select.placeholder")}</span>
        </span>
        <svg
          viewBox="0 0 16 16"
          className={cx(
            "h-3.5 w-3.5 shrink-0 text-text-muted transition-transform duration-150",
            open ? "rotate-180 text-text-secondary" : "",
          )}
          aria-hidden="true"
        >
          <path
            d="M4 6.25 8 10l4-3.75"
            fill="none"
            stroke="currentColor"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="1.5"
          />
        </svg>
      </Button>
      {panelNode}
    </div>
  );
});
