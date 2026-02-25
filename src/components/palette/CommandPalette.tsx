import { useCallback, useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import PaletteInput from "@/components/palette/PaletteInput";
import PaletteList from "@/components/palette/PaletteList";
import PalettePreview from "@/components/palette/PalettePreview";
import { usePaletteStore } from "@/stores/palette.store";

function extractRoute(message: string): string | null {
  if (!message.startsWith("route:")) {
    return null;
  }
  return message.slice("route:".length);
}

export default function CommandPalette() {
  const { t } = useTranslation("palette");
  const navigate = useNavigate();
  const inputRef = useRef<HTMLInputElement>(null);

  const isOpen = usePaletteStore((state) => state.isOpen);
  const query = usePaletteStore((state) => state.query);
  const items = usePaletteStore((state) => state.items);
  const selectedIndex = usePaletteStore((state) => state.selectedIndex);
  const loading = usePaletteStore((state) => state.loading);
  const error = usePaletteStore((state) => state.error);
  const close = usePaletteStore((state) => state.close);
  const toggle = usePaletteStore((state) => state.toggle);
  const moveSelection = usePaletteStore((state) => state.moveSelection);
  const search = usePaletteStore((state) => state.search);
  const setQuery = usePaletteStore((state) => state.setQuery);
  const setSelectedIndex = usePaletteStore((state) => state.setSelectedIndex);
  const executeSelected = usePaletteStore((state) => state.executeSelected);

  const selectedItem = useMemo(() => items[selectedIndex] ?? null, [items, selectedIndex]);

  const handleExecute = useCallback(async () => {
    const result = await executeSelected();
    if (!result) {
      return;
    }

    if (result.ok) {
      const route = extractRoute(result.message);
      if (route) {
        navigate(route);
      }
      close();
    }
  }, [close, executeSelected, navigate]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if ((event.metaKey || event.ctrlKey) && key === "k") {
        event.preventDefault();
        toggle();
        return;
      }

      if (!isOpen) {
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }

      if (event.key === "ArrowDown") {
        event.preventDefault();
        moveSelection(1);
      }

      if (event.key === "ArrowUp") {
        event.preventDefault();
        moveSelection(-1);
      }

      if (event.key === "Enter") {
        event.preventDefault();
        void handleExecute();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [close, handleExecute, isOpen, moveSelection, toggle]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const timer = window.setTimeout(() => {
      void search();
    }, 120);

    return () => window.clearTimeout(timer);
  }, [isOpen, query, search]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    inputRef.current?.focus();

    const handleBlur = () => {
      close();
    };

    window.addEventListener("blur", handleBlur);
    return () => window.removeEventListener("blur", handleBlur);
  }, [close, isOpen]);

  if (!isOpen) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center bg-surface-scrim pt-[10vh]"
      onClick={() => close()}
    >
      <section
        className="w-[min(880px,92vw)] overflow-hidden rounded-xl border border-border-muted bg-surface-overlay shadow-overlay backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)] backdrop-brightness-[var(--glass-brightness)]"
        onClick={(event) => event.stopPropagation()}
      >
        <PaletteInput query={query} loading={loading} onQueryChange={setQuery} inputRef={inputRef} />

        {error ? <div className="p-4 text-[13px] text-danger">{error}</div> : null}

        <div className="grid min-h-[300px] grid-cols-[1fr_0.9fr]">
          <PaletteList
            items={items}
            selectedIndex={selectedIndex}
            onSelect={setSelectedIndex}
            onActivate={(index) => {
              setSelectedIndex(index);
              void handleExecute();
            }}
          />
          <PalettePreview selectedItem={selectedItem} />
        </div>

        <footer className="flex gap-[18px] border-t border-border-muted px-[14px] py-[10px] text-xs text-text-muted">
          <span>{t("command.footer.select")}</span>
          <span>{t("command.footer.execute")}</span>
          <span>{t("command.footer.close")}</span>
        </footer>
      </section>
    </div>
  );
}
