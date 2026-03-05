import { useEffect, useState } from "react";

import type { AppManagerIconKind } from "@/components/app-manager/types";
import { cx } from "@/components/ui/utils";

interface AppEntityIconProps {
  iconKind?: AppManagerIconKind;
  iconValue?: string;
  fallbackIcon?: string;
  imgClassName?: string;
  iconClassName?: string;
}

function normalizeIconifyClass(iconValue?: string): string | null {
  if (!iconValue) {
    return null;
  }

  const value = iconValue.trim();
  if (!value) {
    return null;
  }

  if (!value.startsWith("i-") || !value.includes(":")) {
    return null;
  }

  return value;
}

export function AppEntityIcon(props: AppEntityIconProps) {
  const { iconKind, iconValue, fallbackIcon = "i-noto:desktop-computer", imgClassName, iconClassName } = props;
  const [imageLoadFailed, setImageLoadFailed] = useState(false);
  const resolvedImgClassName = imgClassName
    ? cx("shrink-0 rounded-md object-cover", imgClassName)
    : "h-8 w-8 shrink-0 rounded-md object-cover";
  const resolvedIconClassName = iconClassName
    ? cx("btn-icon shrink-0 text-text-muted", iconClassName)
    : "btn-icon h-8 w-8 shrink-0 text-[1.05rem] text-text-muted";

  useEffect(() => {
    setImageLoadFailed(false);
  }, [iconKind, iconValue]);

  if (iconKind === "raster" && iconValue && !imageLoadFailed) {
    return (
      <img
        src={iconValue}
        alt=""
        className={resolvedImgClassName}
        loading="lazy"
        decoding="async"
        onError={() => setImageLoadFailed(true)}
      />
    );
  }

  const iconClass = normalizeIconifyClass(iconValue) ?? fallbackIcon;
  return <span className={cx(iconClass, resolvedIconClassName)} aria-hidden="true" />;
}
