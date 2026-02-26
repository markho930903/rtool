export function formatBytes(value?: number | null): string {
  if (!value || !Number.isFinite(value) || value <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  const fractionDigits = unitIndex === 0 ? 0 : size >= 100 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(fractionDigits)} ${units[unitIndex]}`;
}

export function toBreadcrumb(path: string): string {
  const normalized = path.trim().replace(/[\\/]+/g, "/");
  if (!normalized) {
    return path;
  }
  const segments = normalized.split("/").filter(Boolean);
  if (segments.length <= 4) {
    return segments.join(" > ");
  }
  return [...segments.slice(0, 1), "...", ...segments.slice(-3)].join(" > ");
}

export function getPathName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+/g, "/");
  if (!normalized) {
    return path;
  }
  const segments = normalized.split("/").filter(Boolean);
  return segments[segments.length - 1] ?? normalized;
}
