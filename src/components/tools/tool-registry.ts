import type { ComponentType } from "react";

import Base64Tool from "@/components/tools/Base64Tool";
import RegexTool from "@/components/tools/RegexTool";
import TimestampTool from "@/components/tools/TimestampTool";

export interface ToolRegistryItem {
  id: string;
  icon: string;
  titleKey: string;
  descriptionKey: string;
  Component: ComponentType;
}

export const TOOL_REGISTRY: ToolRegistryItem[] = [
  {
    id: "base64",
    icon: "i-noto:input-symbols",
    titleKey: "base64.title",
    descriptionKey: "base64.description",
    Component: Base64Tool,
  },
  {
    id: "timestamp",
    icon: "i-noto:mantelpiece-clock",
    titleKey: "timestamp.title",
    descriptionKey: "timestamp.description",
    Component: TimestampTool,
  },
  {
    id: "regex",
    icon: "i-noto:magnifying-glass-tilted-right",
    titleKey: "regex.title",
    descriptionKey: "regex.description",
    Component: RegexTool,
  },
];

const TOOL_REGISTRY_MAP = new Map(TOOL_REGISTRY.map((item) => [item.id, item] as const));

export function normalizeToolId(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const normalized = value.trim().toLowerCase();
  if (!normalized) {
    return null;
  }

  return normalized;
}

export function getToolById(toolId: string | null | undefined): ToolRegistryItem | null {
  const normalized = normalizeToolId(toolId);
  if (!normalized) {
    return null;
  }

  return TOOL_REGISTRY_MAP.get(normalized) ?? null;
}
