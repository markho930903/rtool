import { useTranslation } from "react-i18next";

import SkeletonComposer, { type SkeletonItemSpec } from "@/components/loading/SkeletonComposer";

export type BootOverlayVariant = "main" | "launcher" | "clipboard";

export interface BootOverlayProps {
  variant: BootOverlayVariant;
  visible: boolean;
  title?: string;
  subtitle?: string;
}

interface VariantConfig {
  panelClassName: string;
  radiusClassName: string;
  subtitleKey: string;
  skeletonWidths: string[];
}

const VARIANT_CONFIG: Record<BootOverlayVariant, VariantConfig> = {
  main: {
    panelClassName: "w-[min(560px,86vw)]",
    radiusClassName: "rounded-overlay",
    subtitleKey: "status.preparingMain",
    skeletonWidths: ["68%", "54%", "74%"],
  },
  launcher: {
    panelClassName: "w-[min(440px,84vw)]",
    radiusClassName: "rounded-md",
    subtitleKey: "status.preparingLauncher",
    skeletonWidths: ["66%", "58%"],
  },
  clipboard: {
    panelClassName: "w-[min(520px,84vw)]",
    radiusClassName: "rounded-overlay",
    subtitleKey: "status.preparingClipboard",
    skeletonWidths: ["72%", "62%", "55%"],
  },
};

export default function BootOverlay(props: BootOverlayProps) {
  const { t } = useTranslation("common");
  const config = VARIANT_CONFIG[props.variant];
  const title = props.title ?? t("status.loading");
  const subtitle = props.subtitle ?? t(config.subtitleKey);
  const skeletonItems: SkeletonItemSpec[] = config.skeletonWidths.map((width, index) => ({
    key: `${props.variant}-skeleton-${index}`,
    containerClassName: "rounded-full border border-border-muted/65 bg-surface-soft",
    style: { width },
    body: [
      {
        nodes: [
          {
            kind: "block",
            widthClassName: "w-full",
            heightClassName: "h-2.5",
            className: "bg-transparent",
          },
        ],
      },
    ],
    shimmerDelayMs: index * 90,
  }));

  return (
    <div
      className={[
        "absolute inset-0 z-[68] flex items-center justify-center px-4 transition-opacity duration-160",
        props.visible ? "pointer-events-auto opacity-100" : "pointer-events-none opacity-0",
      ].join(" ")}
      role="status"
      aria-live="polite"
      aria-label={title}
    >
      <div className="absolute inset-0 bg-surface-scrim/26" />

      <div
        className={[
          "relative border border-border-muted bg-surface-overlay/96 p-5 shadow-overlay",
          "backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]",
          config.panelClassName,
          config.radiusClassName,
        ].join(" ")}
      >
        <div className="flex items-center gap-3.5">
          <span className="relative inline-flex h-10 w-10 items-center justify-center rounded-full border border-border-strong bg-surface-soft">
            <span
              className="rtool-boot-motion absolute inset-0 rounded-full border border-accent/42"
              style={{ animation: "rtool-boot-ring-pulse 900ms ease-in-out infinite" }}
              aria-hidden="true"
            />
            <span className="h-1.5 w-1.5 rounded-full bg-accent" aria-hidden="true" />
          </span>

          <div className="min-w-0">
            <div className="text-sm font-semibold text-text-primary">{title}</div>
            <div className="mt-1 text-xs text-text-muted">{subtitle}</div>
          </div>
        </div>

        <SkeletonComposer
          items={skeletonItems}
          tone="plain"
          className="mt-4"
          gapClassName="space-y-2.5"
          shimmerClassName="rtool-boot-shimmer-layer absolute inset-y-0 bg-gradient-to-r from-transparent via-shimmer-highlight/30 to-transparent"
        />
      </div>
    </div>
  );
}
