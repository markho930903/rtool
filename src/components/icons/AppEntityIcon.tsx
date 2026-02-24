import type { AppManagerIconKind } from "@/components/app-manager/types";
import { cx } from "@/components/ui/utils";

interface AppEntityIconProps {
  iconKind?: AppManagerIconKind;
  iconValue?: string;
  fallbackIcon?: string;
  imgClassName?: string;
  iconClassName?: string;
}

export function AppEntityIcon(props: AppEntityIconProps) {
  const { iconKind, iconValue, fallbackIcon = "i-noto:desktop-computer", imgClassName, iconClassName } = props;

  if (iconKind === "raster" && iconValue) {
    return (
      <img
        src={iconValue}
        alt=""
        className={cx("h-8 w-8 shrink-0 rounded-md object-cover", imgClassName)}
        loading="lazy"
        decoding="async"
      />
    );
  }

  const iconClass = iconValue || fallbackIcon;
  return (
    <span
      className={cx("btn-icon h-8 w-8 shrink-0 text-[1.05rem] text-text-muted", iconClass, iconClassName)}
      aria-hidden="true"
    />
  );
}
