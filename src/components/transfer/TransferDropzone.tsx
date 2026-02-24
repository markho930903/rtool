import { useRef, type DragEvent } from "react";
import { useTranslation } from "react-i18next";

import type { TransferFileInput } from "@/components/transfer/types";
import { Button } from "@/components/ui";

interface TransferDropzoneProps {
  pendingFiles: TransferFileInput[];
  onChangeFiles: (files: TransferFileInput[]) => void;
  onSend: () => Promise<void>;
}

type BrowserFile = File & { path?: string; webkitRelativePath?: string };

function normalizeFiles(fileList: FileList): TransferFileInput[] {
  const output: TransferFileInput[] = [];

  for (const file of Array.from(fileList)) {
    const typedFile = file as BrowserFile;
    const path = typedFile.path ?? typedFile.webkitRelativePath ?? file.name;
    const relativePath = typedFile.webkitRelativePath ? typedFile.webkitRelativePath : undefined;

    if (!path.trim()) {
      continue;
    }

    output.push({
      path,
      relativePath,
      compressFolder: false,
    });
  }

  return output;
}

function mergeUnique(current: TransferFileInput[], incoming: TransferFileInput[]): TransferFileInput[] {
  const map = new Map<string, TransferFileInput>();
  for (const item of current) {
    map.set(item.path, item);
  }
  for (const item of incoming) {
    map.set(item.path, item);
  }
  return Array.from(map.values());
}

export default function TransferDropzone(props: TransferDropzoneProps) {
  const { t } = useTranslation("transfer");
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const folderInputRef = useRef<HTMLInputElement | null>(null);

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    const files = event.dataTransfer.files;
    if (!files || files.length === 0) {
      return;
    }
    props.onChangeFiles(mergeUnique(props.pendingFiles, normalizeFiles(files)));
  };

  return (
    <section className="rounded-4 border border-border-muted bg-surface p-4">
      <h2 className="text-sm font-semibold text-text-primary">{t("dropzone.title")}</h2>
      <p className="mt-1 text-xs text-text-secondary">{t("dropzone.desc")}</p>

      <div
        onDragOver={(event) => {
          event.preventDefault();
        }}
        onDrop={onDrop}
        className="mt-3 rounded-3 border border-dashed border-border-muted px-4 py-6 text-center text-xs text-text-secondary"
      >
        {t("dropzone.hint")}
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button
          type="button"
          size="default"
          variant="secondary"
          className="text-ui-xs"
          onClick={() => fileInputRef.current?.click()}
        >
          {t("dropzone.pickFiles")}
        </Button>

        <Button
          type="button"
          size="default"
          variant="secondary"
          className="text-ui-xs"
          onClick={() => folderInputRef.current?.click()}
        >
          {t("dropzone.pickFolder")}
        </Button>

        <Button
          type="button"
          size="default"
          variant="primary"
          className="text-ui-xs"
          onClick={() => {
            void props.onSend();
          }}
        >
          {t("dropzone.send")}
        </Button>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(event) => {
          const files = event.currentTarget.files;
          if (!files || files.length === 0) {
            return;
          }
          props.onChangeFiles(mergeUnique(props.pendingFiles, normalizeFiles(files)));
          event.currentTarget.value = "";
        }}
      />

      <input
        ref={folderInputRef}
        type="file"
        // @ts-expect-error webkitdirectory is supported in Chromium based webview.
        webkitdirectory=""
        multiple
        className="hidden"
        onChange={(event) => {
          const files = event.currentTarget.files;
          if (!files || files.length === 0) {
            return;
          }
          props.onChangeFiles(mergeUnique(props.pendingFiles, normalizeFiles(files)));
          event.currentTarget.value = "";
        }}
      />

      <div className="mt-3 max-h-44 overflow-auto rounded-2 border border-border-muted p-2">
        {props.pendingFiles.length === 0 ? <p className="text-xs text-text-secondary">{t("dropzone.empty")}</p> : null}
        {props.pendingFiles.map((item) => (
          <div key={item.path} className="truncate py-1 text-xs text-text-primary">
            {item.relativePath ?? item.path}
          </div>
        ))}
      </div>
    </section>
  );
}
