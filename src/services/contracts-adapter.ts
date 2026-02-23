function normalize(value: unknown): unknown {
  if (value === null) {
    return undefined;
  }

  if (Array.isArray(value)) {
    return value.map((item) => normalize(item));
  }

  if (typeof value === "object" && value !== null) {
    const output: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      output[key] = normalize(item);
    }
    return output;
  }

  return value;
}

export function normalizeDto<T>(value: unknown): T {
  return normalize(value) as T;
}
