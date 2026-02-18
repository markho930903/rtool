import { forwardRef, useState, type InputHTMLAttributes, type ReactNode } from "react";

import type { ChoiceOrientation, UiSize } from "@/components/ui/types";
import { cx } from "@/components/ui/utils";

type RadioSize = Extract<UiSize, "sm" | "default" | "md">;

export interface RadioProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  size?: RadioSize;
  label?: ReactNode;
  description?: ReactNode;
  wrapperClassName?: string;
}

const sizeClassMap: Record<RadioSize, string> = {
  sm: "h-3.5 w-3.5",
  default: "h-4 w-4",
  md: "h-[18px] w-[18px]",
};

export const Radio = forwardRef<HTMLInputElement, RadioProps>(function Radio(props, ref) {
  const { size = "default", label, description, wrapperClassName, className, disabled, children, ...rest } = props;
  const finalLabel = label ?? children;
  const inputClassName = cx(
    "m-0 shrink-0 rounded-full border border-border-strong bg-surface text-accent accent-accent",
    "outline-none focus-visible:ring-2 focus-visible:ring-accent/55 focus-visible:ring-offset-1 focus-visible:ring-offset-app",
    sizeClassMap[size],
    disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
    className,
  );

  if (!finalLabel && !description) {
    return <input {...rest} ref={ref} type="radio" disabled={disabled} className={inputClassName} />;
  }

  return (
    <label className={cx("inline-flex items-start gap-2 text-sm text-text-secondary", wrapperClassName)}>
      <input {...rest} ref={ref} type="radio" disabled={disabled} className={inputClassName} />
      <span className="inline-flex flex-col gap-0.5">
        {finalLabel ? <span>{finalLabel}</span> : null}
        {description ? <span className="text-xs text-text-muted">{description}</span> : null}
      </span>
    </label>
  );
});

export interface RadioOption {
  label: ReactNode;
  value: string;
  disabled?: boolean;
  description?: ReactNode;
}

export interface RadioGroupProps {
  name: string;
  options: RadioOption[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  orientation?: ChoiceOrientation;
  size?: RadioSize;
  disabled?: boolean;
  className?: string;
  optionClassName?: string;
}

const orientationClassMap: Record<ChoiceOrientation, string> = {
  horizontal: "flex flex-wrap items-center gap-4",
  vertical: "flex flex-col gap-2",
};

export function RadioGroup(props: RadioGroupProps) {
  const {
    name,
    options,
    value,
    defaultValue,
    onValueChange,
    orientation = "vertical",
    size = "default",
    disabled = false,
    className,
    optionClassName,
  } = props;
  const [innerValue, setInnerValue] = useState(defaultValue ?? "");
  const isControlled = value !== undefined;
  const currentValue = isControlled ? value : innerValue;

  const handleChange = (nextValue: string) => {
    if (!isControlled) {
      setInnerValue(nextValue);
    }
    onValueChange?.(nextValue);
  };

  return (
    <div role="radiogroup" className={cx(orientationClassMap[orientation], className)}>
      {options.map((option, index) => {
        const optionDisabled = disabled || option.disabled;
        return (
          <Radio
            key={`${name}-${option.value}-${index}`}
            name={name}
            value={option.value}
            checked={currentValue === option.value}
            onChange={() => handleChange(option.value)}
            disabled={optionDisabled}
            size={size}
            label={option.label}
            description={option.description}
            wrapperClassName={optionClassName}
          />
        );
      })}
    </div>
  );
}
