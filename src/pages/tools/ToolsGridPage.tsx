import { useTranslation } from "react-i18next";

import { TOOL_REGISTRY } from "@/components/tools/tool-registry";
import { Button } from "@/components/ui";

export default function ToolsGridPage() {
  const { t } = useTranslation("tools");

  return (
    <div className="space-y-3">
      <header>
        <h1 className="ui-section-title">{t("page.title")}</h1>
        <p className="mt-1 text-sm text-text-secondary">{t("page.subtitle")}</p>
      </header>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
        {TOOL_REGISTRY.map((item) => (
          <Button
            key={item.id}
            as="link"
            to={`/tools/${item.id}`}
            unstyled
            className="ui-glass-hover group flex h-full min-h-[136px] flex-col rounded-lg border border-border-glass bg-surface-glass-soft p-3 text-left shadow-inset-soft transition-[border-color,transform,box-shadow] duration-180 hover:-translate-y-[1px] hover:border-border-strong focus-visible:ring-2 focus-visible:ring-accent/45"
            aria-label={t(item.titleKey)}
            title={t(item.titleKey)}
          >
            <div className="flex items-center justify-between gap-2">
              <span
                className={`inline-flex h-8 w-8 items-center justify-center rounded-md bg-surface-soft text-[1.15rem] text-text-primary ${item.icon}`}
                aria-hidden="true"
              />
              <span
                className="btn-icon i-lucide:arrow-right text-base text-text-muted transition-colors group-hover:text-text-secondary"
                aria-hidden="true"
              />
            </div>
            <h2 className="mt-2 text-sm font-semibold text-text-primary">{t(item.titleKey)}</h2>
            <p className="mt-1 text-xs leading-5 text-text-secondary">{t(item.descriptionKey)}</p>
          </Button>
        ))}
      </div>
    </div>
  );
}
