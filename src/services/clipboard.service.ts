import type {
  ClipboardFilterDto,
  ClipboardItemDto,
  ClipboardWindowModeAppliedDto,
  CommandRequestDto,
} from "@/contracts";
import { invokeWithLog } from "@/services/invoke";

export interface ClipboardFilterInput {
  query?: string | null;
  itemType?: string | null;
  onlyPinned?: boolean | null;
  limit?: number | null;
}

function invokeClipboard<T>(kind: string, payload?: Record<string, unknown>): Promise<T> {
  const request: CommandRequestDto = { kind };
  if (payload !== undefined) {
    request.payload = payload;
  }
  return invokeWithLog<T>("clipboard_handle", { request });
}

export async function clipboardList(filter?: ClipboardFilterInput): Promise<ClipboardItemDto[]> {
  const normalizedFilter: ClipboardFilterDto | undefined = filter
    ? {
        query: filter.query ?? null,
        itemType: filter.itemType ?? null,
        onlyPinned: filter.onlyPinned ?? null,
        limit: filter.limit ?? null,
      }
    : undefined;
  return invokeClipboard<ClipboardItemDto[]>("list", { filter: normalizedFilter });
}

export async function clipboardPin(id: string, pinned: boolean): Promise<void> {
  await invokeClipboard<void>("pin", { id, pinned });
}

export async function clipboardDelete(id: string): Promise<void> {
  await invokeClipboard<void>("delete", { id });
}

export async function clipboardClearAll(): Promise<void> {
  await invokeClipboard<void>("clear_all");
}

export async function clipboardSaveText(text: string): Promise<ClipboardItemDto> {
  return invokeClipboard<ClipboardItemDto>("save_text", { text });
}

export async function clipboardWindowSetMode(compact: boolean): Promise<void> {
  await invokeClipboard<void>("window_set_mode", { compact });
}

export async function clipboardWindowApplyMode(
  compact: boolean,
): Promise<ClipboardWindowModeAppliedDto> {
  return invokeClipboard<ClipboardWindowModeAppliedDto>("window_apply_mode", { compact });
}

export async function clipboardCopyBack(id: string): Promise<void> {
  await invokeClipboard<void>("copy_back", { id });
}

export async function clipboardCopyFilePaths(id: string): Promise<void> {
  await invokeClipboard<void>("copy_file_paths", { id });
}

export async function clipboardCopyImageBack(id: string): Promise<void> {
  await invokeClipboard<void>("copy_image_back", { id });
}
