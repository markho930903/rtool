export type AppPathType = "file" | "directory";

const FILE_FALLBACK_ICON = "i-noto:page-facing-up";
const DIRECTORY_ICON = "i-noto:file-folder";
const DIRECTORY_SUFFIX_HINTS = [
  ".app",
  ".bundle",
  ".framework",
  ".appex",
  ".plugin",
  ".kext",
  ".savedstate",
  ".photoslibrary",
];
const FILE_EXTENSION_HINTS = new Set([
  "plist",
  "lnk",
  "json",
  "xml",
  "toml",
  "yaml",
  "yml",
  "ini",
  "conf",
  "cfg",
  "log",
  "txt",
  "db",
  "sqlite",
  "sqlite3",
]);
const RESIDUE_FILE_PREFERRED_KINDS = new Set([
  "preferences",
  "startup",
  "app_script",
  "launch_agent",
  "launch_daemon",
  "registry_value",
]);
const RESIDUE_DIRECTORY_PREFERRED_KINDS = new Set([
  "install",
  "app_support",
  "cache",
  "logs",
  "container",
  "group_container",
  "saved_state",
  "webkit_data",
  "helper_tool",
  "app_data",
  "main_app",
  "registry_key",
]);

export function normalizePathType(pathType?: string | null): AppPathType {
  const normalized = pathType?.trim().toLowerCase();
  if (normalized === "file") {
    return "file";
  }
  if (normalized === "directory" || normalized === "dir" || normalized === "folder") {
    return "directory";
  }
  return "directory";
}

export function getFileExtension(path: string | undefined): string | null {
  if (!path) {
    return null;
  }

  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");
  const fileName = parts.length > 0 ? parts[parts.length - 1] : "";
  if (!fileName || fileName === "." || fileName === "..") {
    return null;
  }

  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === fileName.length - 1) {
    return null;
  }

  return fileName.slice(dotIndex + 1).toLowerCase();
}

function getPathLeaf(path: string | undefined): string | null {
  if (!path) {
    return null;
  }
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");
  const fileName = parts.length > 0 ? parts[parts.length - 1] : "";
  if (!fileName || fileName === "." || fileName === "..") {
    return null;
  }
  return fileName;
}

function hasDirectorySuffixHint(path: string | undefined): boolean {
  const leaf = getPathLeaf(path);
  if (!leaf) {
    return false;
  }
  const lower = leaf.toLowerCase();
  return DIRECTORY_SUFFIX_HINTS.some((suffix) => lower.endsWith(suffix));
}

function hasFileExtensionHint(ext: string | null): boolean {
  if (!ext) {
    return false;
  }
  return FILE_EXTENSION_HINTS.has(ext.toLowerCase());
}

function isFilePreferredResidueKind(kind?: string | null): boolean {
  if (!kind) {
    return false;
  }
  return RESIDUE_FILE_PREFERRED_KINDS.has(kind.trim().toLowerCase());
}

function isDirectoryPreferredResidueKind(kind?: string | null): boolean {
  if (!kind) {
    return false;
  }
  return RESIDUE_DIRECTORY_PREFERRED_KINDS.has(kind.trim().toLowerCase());
}

export function resolveFileIconByExtension(ext: string | null): string {
  if (!ext) {
    return FILE_FALLBACK_ICON;
  }

  if (ext === "pdf") {
    return "i-noto:page-facing-up";
  }

  if (ext === "doc" || ext === "docx" || ext === "rtf") {
    return "i-noto:memo";
  }

  if (ext === "xls" || ext === "xlsx" || ext === "csv") {
    return "i-noto:bar-chart";
  }

  if (ext === "ppt" || ext === "pptx") {
    return "i-noto:rolled-up-newspaper";
  }

  if (
    ext === "png" ||
    ext === "jpg" ||
    ext === "jpeg" ||
    ext === "webp" ||
    ext === "gif" ||
    ext === "bmp" ||
    ext === "svg"
  ) {
    return "i-noto:framed-picture";
  }

  if (ext === "mp4" || ext === "mov" || ext === "mkv" || ext === "avi" || ext === "webm") {
    return "i-noto:film-projector";
  }

  if (ext === "mp3" || ext === "wav" || ext === "flac" || ext === "aac" || ext === "ogg") {
    return "i-noto:musical-notes";
  }

  if (ext === "zip" || ext === "rar" || ext === "7z" || ext === "tar" || ext === "gz") {
    return DIRECTORY_ICON;
  }

  if (
    ext === "json" ||
    ext === "yaml" ||
    ext === "yml" ||
    ext === "toml" ||
    ext === "plist" ||
    ext === "xml" ||
    ext === "ini" ||
    ext === "md" ||
    ext === "txt"
  ) {
    return "i-noto:scroll";
  }

  if (
    ext === "rs" ||
    ext === "ts" ||
    ext === "tsx" ||
    ext === "js" ||
    ext === "jsx" ||
    ext === "py" ||
    ext === "go" ||
    ext === "java" ||
    ext === "c" ||
    ext === "cpp" ||
    ext === "h" ||
    ext === "hpp"
  ) {
    return "i-noto:desktop-computer";
  }

  if (ext === "sql") {
    return "i-noto:floppy-disk";
  }

  return FILE_FALLBACK_ICON;
}

export function resolvePathIcon(path: string, pathType?: string | null): string {
  const normalizedPathType = normalizePathType(pathType);
  if (normalizedPathType === "directory") {
    return DIRECTORY_ICON;
  }
  return resolveFileIconByExtension(getFileExtension(path));
}

export function resolveResiduePathIcon(path: string, pathType?: string | null, kind?: string | null): string {
  const normalizedPathType = normalizePathType(pathType);
  const ext = getFileExtension(path);
  if (normalizedPathType === "file") {
    return resolveFileIconByExtension(ext);
  }
  if (hasDirectorySuffixHint(path)) {
    return DIRECTORY_ICON;
  }
  if (isDirectoryPreferredResidueKind(kind)) {
    return DIRECTORY_ICON;
  }
  if (normalizedPathType === "directory" && hasFileExtensionHint(ext)) {
    return resolveFileIconByExtension(ext);
  }
  if (isFilePreferredResidueKind(kind)) {
    return FILE_FALLBACK_ICON;
  }
  return DIRECTORY_ICON;
}
