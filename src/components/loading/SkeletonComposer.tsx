import type { CSSProperties } from "react";

import { cx } from "@/components/ui/utils";

export type SkeletonTone = "soft" | "glass" | "plain";
export type SkeletonNodeKind = "line" | "block" | "circle";

export interface SkeletonNodeSpec {
  key?: string;
  kind?: SkeletonNodeKind;
  widthClassName?: string;
  heightClassName?: string;
  offsetTopClassName?: string;
  className?: string;
  style?: CSSProperties;
}

export interface SkeletonRegionSpec {
  key?: string;
  containerClassName?: string;
  nodes: SkeletonNodeSpec[];
  style?: CSSProperties;
}

export interface SkeletonItemSpec {
  key?: string;
  containerClassName?: string;
  leading?: SkeletonRegionSpec[];
  body: SkeletonRegionSpec[];
  trailing?: SkeletonRegionSpec[];
  shimmerDelayMs?: number;
  style?: CSSProperties;
}

export interface SkeletonComposerProps {
  items: SkeletonItemSpec[];
  className?: string;
  gapClassName?: string;
  itemClassName?: string;
  itemSurfaceClassName?: string;
  tone?: SkeletonTone;
  animated?: boolean;
  shimmerDurationMs?: number;
  shimmerBandWidthPercent?: number;
  shimmerClassName?: string;
  ariaHidden?: boolean;
}

interface ToneStyle {
  itemClassName: string;
  lineClassName: string;
  blockClassName: string;
}

const TONE_STYLE_MAP: Record<SkeletonTone, ToneStyle> = {
  soft: {
    itemClassName: "rounded-md border border-border-muted/65 px-2.5 py-2.5 shadow-inset-soft",
    lineClassName: "bg-border-muted/60",
    blockClassName: "bg-border-muted/60",
  },
  glass: {
    itemClassName: "rounded-lg border border-border-glass px-3 py-2.5 shadow-inset-soft",
    lineClassName: "bg-border-muted/58",
    blockClassName: "bg-border-muted/58",
  },
  plain: {
    itemClassName: "rounded-md border border-transparent p-0 shadow-none",
    lineClassName: "bg-border-muted/55",
    blockClassName: "bg-border-muted/55",
  },
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function getDuration(durationMs?: number): number {
  if (!durationMs || !Number.isFinite(durationMs)) {
    return 1200;
  }
  return clamp(Math.round(durationMs), 400, 4000);
}

function getBandWidth(bandWidth?: number): number {
  if (!bandWidth || !Number.isFinite(bandWidth)) {
    return 45;
  }
  return clamp(bandWidth, 18, 80);
}

function buildShimmerStyle(durationMs: number, delayMs: number, bandWidth: number): CSSProperties {
  return {
    left: `-${bandWidth}%`,
    width: `${bandWidth}%`,
    animation: `rtool-boot-shimmer ${durationMs}ms linear infinite`,
    animationDelay: `${delayMs}ms`,
  };
}

function getWidthClassName(node: SkeletonNodeSpec, kind: SkeletonNodeKind): string | undefined {
  if (node.widthClassName) {
    return node.widthClassName;
  }
  if (node.style?.width !== undefined) {
    return undefined;
  }
  if (kind === "line") {
    return "w-[60%]";
  }
  if (kind === "circle") {
    return "w-5";
  }
  return "w-12";
}

function getHeightClassName(node: SkeletonNodeSpec, kind: SkeletonNodeKind): string | undefined {
  if (node.heightClassName) {
    return node.heightClassName;
  }
  if (node.style?.height !== undefined) {
    return undefined;
  }
  if (kind === "line") {
    return "h-2.5";
  }
  if (kind === "circle") {
    return "h-5";
  }
  return "h-3";
}

function renderNode(
  node: SkeletonNodeSpec,
  nodeIndex: number,
  toneStyle: ToneStyle,
  itemKey: string,
  regionKey: string,
) {
  const kind = node.kind ?? "line";
  const widthClassName = getWidthClassName(node, kind);
  const heightClassName = getHeightClassName(node, kind);
  const offsetTopClassName = node.offsetTopClassName ?? (kind === "line" && nodeIndex > 0 ? "mt-1.5" : undefined);
  const shapeClassName = kind === "circle" ? "rounded-full" : "rounded";
  const colorClassName = kind === "line" ? toneStyle.lineClassName : toneStyle.blockClassName;
  return (
    <div
      key={node.key ?? `${itemKey}-${regionKey}-node-${nodeIndex}`}
      className={cx(
        shapeClassName,
        colorClassName,
        widthClassName,
        heightClassName,
        offsetTopClassName,
        node.className,
      )}
      style={node.style}
    />
  );
}

function renderRegionGroup(
  regions: SkeletonRegionSpec[] | undefined,
  toneStyle: ToneStyle,
  itemKey: string,
  slot: "leading" | "body" | "trailing",
) {
  if (!regions || regions.length === 0) {
    return null;
  }

  return regions.map((region, regionIndex) => {
    const regionKey = region.key ?? `${itemKey}-${slot}-${regionIndex}`;
    return (
      <div
        key={regionKey}
        className={cx(slot === "body" ? undefined : "shrink-0", region.containerClassName)}
        style={region.style}
      >
        {region.nodes.map((node, nodeIndex) => renderNode(node, nodeIndex, toneStyle, itemKey, regionKey))}
      </div>
    );
  });
}

export default function SkeletonComposer(props: SkeletonComposerProps) {
  const tone = props.tone ?? "soft";
  const toneStyle = TONE_STYLE_MAP[tone];
  const animated = props.animated ?? true;
  const durationMs = getDuration(props.shimmerDurationMs);
  const bandWidth = getBandWidth(props.shimmerBandWidthPercent);
  const ariaHidden = props.ariaHidden ?? true;
  const itemSurfaceClassName =
    props.itemSurfaceClassName ??
    (tone === "glass" ? "bg-surface-glass-soft" : tone === "soft" ? "bg-surface" : "bg-transparent");
  const shimmerClassName =
    props.shimmerClassName ??
    "rtool-boot-shimmer-layer absolute inset-y-0 bg-gradient-to-r from-transparent via-shimmer-highlight/26 to-transparent";

  if (props.items.length === 0) {
    return null;
  }

  return (
    <div
      className={cx(props.gapClassName ?? "space-y-2", props.className)}
      aria-hidden={ariaHidden ? "true" : undefined}
    >
      {props.items.map((item, itemIndex) => {
        const itemKey = item.key ?? `skeleton-item-${itemIndex}`;
        const shimmerDelayMs = item.shimmerDelayMs ?? itemIndex * 80;
        return (
          <div
            key={itemKey}
            className={cx(
              "relative overflow-hidden transition-colors duration-200",
              toneStyle.itemClassName,
              itemSurfaceClassName,
              props.itemClassName,
              item.containerClassName,
            )}
            style={item.style}
          >
            <div className="flex items-start gap-2">
              {renderRegionGroup(item.leading, toneStyle, itemKey, "leading")}

              <div className="min-w-0 flex-1">{renderRegionGroup(item.body, toneStyle, itemKey, "body")}</div>

              {renderRegionGroup(item.trailing, toneStyle, itemKey, "trailing")}
            </div>

            {animated ? (
              <span className={shimmerClassName} style={buildShimmerStyle(durationMs, shimmerDelayMs, bandWidth)} />
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
