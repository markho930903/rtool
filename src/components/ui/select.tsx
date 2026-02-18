import {
  Children,
  forwardRef,
  isValidElement,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
  type SelectHTMLAttributes,
} from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { SelectVariant } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type SelectSize = "sm" | "md";

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
    "w-full min-h-[38px] rounded-md border border-border-muted bg-surface px-3 text-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-strong hover:bg-surface-soft",
  tool: "min-h-[36px] rounded-md border border-border-muted bg-surface px-2.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-strong hover:bg-surface-soft",
  clipboard:
    "min-h-[36px] rounded-md border border-border-muted bg-surface px-2.5 text-ui-sm leading-ui-sm text-text-primary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-strong hover:bg-surface-soft",
  theme:
    "h-8 rounded-lg border border-border-muted bg-surface px-2 text-xs text-text-secondary shadow-inset-soft outline-none transition-[border-color,background-color,box-shadow] duration-150 hover:border-border-strong hover:bg-surface-soft",
};

const panelClassMap: Record<SelectVariant, string> = {
  default:
    "left-0 mt-1.5 max-h-64 w-full overflow-y-auto rounded-lg border border-border-muted bg-surface-overlay p-1 shadow-overlay backdrop-blur-md backdrop-saturate-130",
  tool: "left-0 mt-1.5 max-h-64 min-w-full overflow-y-auto rounded-lg border border-border-muted bg-surface-overlay p-1 shadow-overlay backdrop-blur-md backdrop-saturate-130",
  clipboard:
    "left-0 mt-1.5 max-h-64 min-w-full overflow-y-auto rounded-lg border border-border-muted bg-surface-overlay p-1 shadow-overlay backdrop-blur-md backdrop-saturate-130",
  theme:
    "right-0 mt-1.5 max-h-64 min-w-full overflow-y-auto rounded-lg border border-border-muted bg-surface-overlay p-1 shadow-overlay backdrop-blur-md backdrop-saturate-130",
};

const optionClassMap: Record<SelectVariant, string> = {
  default: "min-h-[34px] rounded-md px-2.5 text-sm",
  tool: "min-h-[32px] rounded-md px-2.5 text-ui-sm leading-ui-sm",
  clipboard: "min-h-[32px] rounded-md px-2.5 text-ui-sm leading-ui-sm",
  theme: "min-h-[30px] rounded-md px-2 text-xs",
};

const sizeClassMap: Record<SelectSize, string> = {
  sm: "text-xs",
  md: "",
};

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
    size = "md",
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

  const rootRef = useRef<HTMLDivElement>(null);
  const hiddenSelectRef = useRef<HTMLSelectElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLLIElement | null>>([]);
  const listboxId = useId();

  const selectedOption = options.find((option) => option.value === selectedValue) ?? options[0] ?? null;

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
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };

    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

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

  if (multiple) {
    const selectClassName = cx(
      triggerClassMap[variant],
      sizeClassMap[size],
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
    sizeClassMap[size],
    invalid ? "border-danger focus-visible:border-danger focus-visible:ring-danger/25" : null,
    disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
    className,
  );

  const panelClassName = cx("absolute z-30", panelClassMap[variant]);

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

      {open ? (
        <ul
          id={listboxId}
          role="listbox"
          className={panelClassName}
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
                    : "cursor-pointer text-text-secondary hover:bg-surface-soft hover:text-text-primary",
                  isActive && !option.disabled ? "bg-surface-soft text-text-primary" : null,
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
        </ul>
      ) : null}
    </div>
  );
});
