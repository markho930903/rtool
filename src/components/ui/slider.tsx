import {
  forwardRef,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type FocusEvent,
  type InputHTMLAttributes,
  type KeyboardEvent,
  type PointerEvent,
  type ReactNode,
} from "react";

import type { SliderSize, SliderTooltipMode, SliderVariant } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

const DEFAULT_MIN = 0;
const DEFAULT_MAX = 100;
const DEFAULT_STEP = 1;
const DEFAULT_TOOLTIP_MODE: SliderTooltipMode = "dragging";

type NativeSliderProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "size" | "value" | "defaultValue" | "min" | "max" | "step" | "onChange"
>;

export interface SliderMark {
  value: number;
  label?: ReactNode;
  disabled?: boolean;
}

export interface SliderProps extends NativeSliderProps {
  value?: number;
  defaultValue?: number;
  min?: number;
  max?: number;
  step?: number;
  variant?: SliderVariant;
  size?: SliderSize;
  invalid?: boolean;
  showTooltip?: SliderTooltipMode;
  showValue?: boolean;
  marks?: SliderMark[];
  formatValue?: (value: number) => string;
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void;
  onValueChange?: (value: number) => void;
  onValueCommit?: (value: number) => void;
  wrapperClassName?: string;
  trackClassName?: string;
  fillClassName?: string;
  thumbClassName?: string;
  marksClassName?: string;
  valueClassName?: string;
}

interface SliderVisualSize {
  root: string;
  track: string;
  thumb: string;
}

const rootVariantClassMap: Record<SliderVariant, string> = {
  default: "w-full",
  tool: "w-full",
  theme: "w-full",
};

const visualSizeClassMap: Record<SliderSize, SliderVisualSize> = {
  sm: {
    root: "h-7",
    track: "h-1.5",
    thumb: "h-3.5 w-3.5",
  },
  default: {
    root: "h-8",
    track: "h-2",
    thumb: "h-4 w-4",
  },
  md: {
    root: "h-9",
    track: "h-2.5",
    thumb: "h-5 w-5",
  },
};

const tooltipOffsetStyle = { transform: "translate(-50%, -130%)" } as const;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function toFiniteNumber(value: number | undefined, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }
  return value;
}

function normalizeRange(min: number | undefined, max: number | undefined): { min: number; max: number } {
  const normalizedMin = toFiniteNumber(min, DEFAULT_MIN);
  const fallbackMax = normalizedMin < DEFAULT_MAX ? DEFAULT_MAX : normalizedMin + DEFAULT_STEP;
  const rawMax = toFiniteNumber(max, fallbackMax);
  if (rawMax <= normalizedMin) {
    return { min: normalizedMin, max: normalizedMin + DEFAULT_STEP };
  }
  return { min: normalizedMin, max: rawMax };
}

function countStepPrecision(step: number): number {
  const stepText = step.toString();
  const decimalIndex = stepText.indexOf(".");
  if (decimalIndex < 0) {
    return 0;
  }
  return stepText.length - decimalIndex - 1;
}

function alignToStep(value: number, min: number, step: number): number {
  const precision = countStepPrecision(step);
  const stepCount = Math.round((value - min) / step);
  const aligned = min + stepCount * step;
  if (precision <= 0) {
    return aligned;
  }
  const factor = 10 ** precision;
  return Math.round(aligned * factor) / factor;
}

function normalizeStep(step: number | undefined): number {
  const normalizedStep = toFiniteNumber(step, DEFAULT_STEP);
  if (normalizedStep <= 0) {
    return DEFAULT_STEP;
  }
  return normalizedStep;
}

function normalizeSliderValue(value: number, min: number, max: number, step: number): number {
  return clamp(alignToStep(value, min, step), min, max);
}

function isCommitKey(key: string): boolean {
  return (
    key === "ArrowLeft" ||
    key === "ArrowRight" ||
    key === "ArrowUp" ||
    key === "ArrowDown" ||
    key === "Home" ||
    key === "End" ||
    key === "PageUp" ||
    key === "PageDown"
  );
}

export const Slider = forwardRef<HTMLInputElement, SliderProps>(function Slider(props, ref) {
  const {
    value,
    defaultValue,
    min,
    max,
    step,
    variant = "default",
    size = "default",
    invalid = false,
    showTooltip = DEFAULT_TOOLTIP_MODE,
    showValue = false,
    marks,
    formatValue,
    onChange,
    onValueChange,
    onValueCommit,
    className,
    wrapperClassName,
    trackClassName,
    fillClassName,
    thumbClassName,
    marksClassName,
    valueClassName,
    disabled,
    onPointerDown,
    onKeyUp,
    onBlur,
    ...rest
  } = props;

  const normalizedRange = useMemo(() => normalizeRange(min, max), [max, min]);
  const normalizedStep = useMemo(() => normalizeStep(step), [step]);
  const initialValue = useMemo(
    () =>
      normalizeSliderValue(
        toFiniteNumber(defaultValue, normalizedRange.min),
        normalizedRange.min,
        normalizedRange.max,
        normalizedStep,
      ),
    [defaultValue, normalizedRange.max, normalizedRange.min, normalizedStep],
  );

  const isControlled = typeof value === "number";
  const [internalValue, setInternalValue] = useState<number>(initialValue);
  const [isDragging, setIsDragging] = useState(false);
  const currentValue = isControlled
    ? normalizeSliderValue(value, normalizedRange.min, normalizedRange.max, normalizedStep)
    : internalValue;
  const displayValue = formatValue ? formatValue(currentValue) : String(currentValue);

  const currentValueRef = useRef(currentValue);
  useEffect(() => {
    currentValueRef.current = currentValue;
  }, [currentValue]);

  useEffect(() => {
    if (isControlled) {
      return;
    }
    setInternalValue((previous) =>
      normalizeSliderValue(previous, normalizedRange.min, normalizedRange.max, normalizedStep),
    );
  }, [isControlled, normalizedRange.max, normalizedRange.min, normalizedStep]);

  const commitCurrentValue = useCallback(() => {
    onValueCommit?.(currentValueRef.current);
  }, [onValueCommit]);

  useEffect(() => {
    if (!isDragging || disabled) {
      return;
    }

    const handlePointerEnd = () => {
      setIsDragging(false);
      commitCurrentValue();
    };

    window.addEventListener("pointerup", handlePointerEnd, true);
    window.addEventListener("pointercancel", handlePointerEnd, true);

    return () => {
      window.removeEventListener("pointerup", handlePointerEnd, true);
      window.removeEventListener("pointercancel", handlePointerEnd, true);
    };
  }, [commitCurrentValue, disabled, isDragging]);

  const rangeSpan = normalizedRange.max - normalizedRange.min;
  const progressPercent = rangeSpan <= 0 ? 0 : ((currentValue - normalizedRange.min) / rangeSpan) * 100;

  const normalizedMarks = useMemo(() => {
    if (!marks || marks.length === 0) {
      return [];
    }

    return marks
      .map((mark, index) => ({ mark, index }))
      .filter(({ mark }) => Number.isFinite(mark.value))
      .map(({ mark, index }) => {
        const clamped = clamp(mark.value, normalizedRange.min, normalizedRange.max);
        const percent = rangeSpan <= 0 ? 0 : ((clamped - normalizedRange.min) / rangeSpan) * 100;

        return {
          index,
          value: clamped,
          percent,
          disabled: Boolean(mark.disabled),
          label: mark.label,
        };
      });
  }, [marks, normalizedRange.max, normalizedRange.min, rangeSpan]);

  const hasMarkLabels = normalizedMarks.some((mark) => mark.label !== undefined && mark.label !== null);
  const shouldShowTooltip = showTooltip === "always" || (showTooltip === "dragging" && isDragging);
  const visualSize = visualSizeClassMap[size];

  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const nextValue = normalizeSliderValue(
        Number.parseFloat(event.currentTarget.value),
        normalizedRange.min,
        normalizedRange.max,
        normalizedStep,
      );

      if (!isControlled) {
        setInternalValue(nextValue);
      }

      if (Number.parseFloat(event.currentTarget.value) !== nextValue) {
        event.currentTarget.value = String(nextValue);
      }

      onChange?.(event);
      onValueChange?.(nextValue);
    },
    [isControlled, normalizedRange.max, normalizedRange.min, normalizedStep, onChange, onValueChange],
  );

  const handlePointerStart = useCallback(
    (event: PointerEvent<HTMLInputElement>) => {
      if (!disabled) {
        setIsDragging(true);
      }
      onPointerDown?.(event);
    },
    [disabled, onPointerDown],
  );

  const handleKeyUp = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      if (isCommitKey(event.key)) {
        commitCurrentValue();
      }
      onKeyUp?.(event);
    },
    [commitCurrentValue, onKeyUp],
  );

  const handleBlur = useCallback(
    (event: FocusEvent<HTMLInputElement>) => {
      setIsDragging(false);
      commitCurrentValue();
      onBlur?.(event);
    },
    [commitCurrentValue, onBlur],
  );

  return (
    <div className={cx("inline-flex w-full items-center gap-3", rootVariantClassMap[variant], wrapperClassName)}>
      <div className={cx("relative min-w-0 flex-1", className)}>
        <div className={cx("relative", visualSize.root)}>
          <input
            {...rest}
            ref={ref}
            type="range"
            min={normalizedRange.min}
            max={normalizedRange.max}
            step={normalizedStep}
            value={currentValue}
            disabled={disabled}
            aria-invalid={invalid || undefined}
            className={cx(
              "peer absolute inset-0 z-20 m-0 h-full w-full appearance-none bg-transparent outline-none opacity-0",
              disabled ? "cursor-not-allowed" : "cursor-pointer",
            )}
            onChange={handleChange}
            onPointerDown={handlePointerStart}
            onKeyUp={handleKeyUp}
            onBlur={handleBlur}
          />

          <div className="pointer-events-none absolute inset-0 flex items-center">
            <div
              className={cx(
                "relative w-full overflow-hidden rounded-full border border-border-glass bg-surface-glass-soft shadow-inset-soft",
                visualSize.track,
                invalid ? "border-danger/80" : null,
                trackClassName,
              )}
            >
              <div
                aria-hidden="true"
                className={cx(
                  "absolute inset-y-0 left-0 rounded-full bg-accent transition-[width,background-color] duration-100 ease-out",
                  invalid ? "bg-danger" : null,
                  fillClassName,
                )}
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          </div>

          <div
            className="pointer-events-none absolute inset-y-0 left-0 z-10 flex items-center"
            style={{ left: `${progressPercent}%` }}
          >
            <span
              aria-hidden="true"
              className={cx(
                "block -translate-x-1/2 rounded-full border border-border-glass-strong bg-surface-glass-strong shadow-surface",
                "transition-[transform,border-color,box-shadow,background-color] duration-150 ease-out",
                "peer-focus-visible:ring-2 peer-focus-visible:ring-accent/55 peer-focus-visible:ring-offset-1 peer-focus-visible:ring-offset-app",
                isDragging ? "scale-110 border-accent" : null,
                invalid ? "border-danger peer-focus-visible:ring-danger/45" : null,
                disabled ? "opacity-70" : null,
                visualSize.thumb,
                thumbClassName,
              )}
            />
          </div>

          {shouldShowTooltip ? (
            <div
              className="pointer-events-none absolute left-0 top-0 z-30"
              style={{ left: `${progressPercent}%`, ...tooltipOffsetStyle }}
            >
              <span
                className={cx(
                  "inline-flex items-center rounded-md border border-border-glass bg-surface-glass-strong px-2 py-0.5",
                  "text-xs leading-4 text-text-primary shadow-overlay",
                  invalid ? "text-danger" : null,
                )}
              >
                {displayValue}
              </span>
            </div>
          ) : null}
        </div>

        {normalizedMarks.length > 0 ? (
          <div className={cx("mt-1.5", marksClassName)}>
            <div className="relative h-2">
              {normalizedMarks.map((mark) => (
                <span
                  key={`${mark.value}-${mark.index}`}
                  aria-hidden="true"
                  className={cx(
                    "absolute top-0 block h-2 w-px -translate-x-1/2 rounded-full bg-border-strong",
                    mark.disabled ? "bg-border-muted" : null,
                  )}
                  style={{ left: `${mark.percent}%` }}
                />
              ))}
            </div>

            {hasMarkLabels ? (
              <div className="relative mt-1 h-4 text-xs leading-4 text-text-muted">
                {normalizedMarks.map((mark) =>
                  mark.label ? (
                    <span
                      key={`label-${mark.value}-${mark.index}`}
                      className={cx("absolute -translate-x-1/2 whitespace-nowrap", mark.disabled ? "opacity-70" : null)}
                      style={{ left: `${mark.percent}%` }}
                    >
                      {mark.label}
                    </span>
                  ) : null,
                )}
              </div>
            ) : null}
          </div>
        ) : null}
      </div>

      {showValue ? (
        <span className={cx("shrink-0 text-xs text-text-secondary tabular-nums", valueClassName)}>{displayValue}</span>
      ) : null}
    </div>
  );
});
