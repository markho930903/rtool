export type ClipboardType = "text" | "link" | "image" | "file" | "code" | "color";

export interface ClipboardItem {
  id: string;
  contentKey: string;
  itemType: ClipboardType | string;
  plainText: string;
  sourceApp: string | null;
  previewPath: string | null;
  previewDataUrl: string | null;
  createdAt: number;
  pinned: boolean;
}

export interface ClipboardFilter {
  query?: string;
  itemType?: string;
  onlyPinned?: boolean;
  limit?: number;
}

export interface ClipboardSyncPayload {
  upsert?: ClipboardItem[];
  removedIds?: string[];
  clearAll?: boolean;
  reason?: string;
}
