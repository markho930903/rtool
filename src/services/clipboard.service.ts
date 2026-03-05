import type {
  ClipboardFilterDto,
  ClipboardRequestDto,
  ClipboardImageExportResultDto,
  ClipboardItemDto,
  ClipboardWindowModeAppliedDto,
} from "@/contracts";
import { invokeFeature } from "@/services/invoke";

export interface ClipboardFilterInput {
  query?: string | null;
  itemType?: string | null;
  onlyPinned?: boolean | null;
  limit?: number | null;
}

function invokeClipboard<T>(request: ClipboardRequestDto): Promise<T> {
  return invokeFeature<T>("clipboard", request);
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
  return invokeClipboard<ClipboardItemDto[]>({
    kind: "list",
    payload: { filter: normalizedFilter },
  });
}

export async function clipboardPin(id: string, pinned: boolean): Promise<void> {
  await invokeClipboard<void>({ kind: "pin", payload: { id, pinned } });
}

export async function clipboardDelete(id: string): Promise<void> {
  await invokeClipboard<void>({ kind: "delete", payload: { id } });
}

export async function clipboardClearAll(): Promise<void> {
  await invokeClipboard<void>({ kind: "clear_all" });
}

export async function clipboardSaveText(text: string): Promise<ClipboardItemDto> {
  return invokeClipboard<ClipboardItemDto>({ kind: "save_text", payload: { text } });
}

export async function clipboardWindowSetMode(compact: boolean): Promise<void> {
  await invokeClipboard<void>({ kind: "window_set_mode", payload: { compact } });
}

export async function clipboardWindowApplyMode(
  compact: boolean,
): Promise<ClipboardWindowModeAppliedDto> {
  return invokeClipboard<ClipboardWindowModeAppliedDto>({
    kind: "window_apply_mode",
    payload: { compact },
  });
}

export async function clipboardCopyBack(id: string): Promise<void> {
  await invokeClipboard<void>({ kind: "copy_back", payload: { id } });
}

export async function clipboardCopyFilePaths(id: string): Promise<void> {
  await invokeClipboard<void>({ kind: "copy_file_paths", payload: { id } });
}

export async function clipboardCopyImageBack(id: string): Promise<void> {
  await invokeClipboard<void>({ kind: "copy_image_back", payload: { id } });
}

export async function clipboardExportImage(id: string): Promise<ClipboardImageExportResultDto> {
  return invokeClipboard<ClipboardImageExportResultDto>({ kind: "export_image", payload: { id } });
}
