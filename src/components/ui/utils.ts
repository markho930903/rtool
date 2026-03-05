export type ClassNameValue = string | null | undefined | false;

export function cx(...values: ClassNameValue[]): string {
  return values.filter(Boolean).join(" ");
}
