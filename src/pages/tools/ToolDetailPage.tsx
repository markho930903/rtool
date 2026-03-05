import { useTranslation } from "react-i18next";
import { useParams } from "react-router";

import { getToolById } from "@/components/tools/tool-registry";
import { Button } from "@/components/ui";

export default function ToolDetailPage() {
  const { t } = useTranslation("tools");
  const { toolId } = useParams();
  const tool = getToolById(toolId);

  if (!tool) {
    return (
      <div className="space-y-3">
        <header>
          <h1 className="ui-section-title">{t("page.title")}</h1>
          <p className="mt-1 text-sm text-text-secondary">{t("detail.notFoundDescription")}</p>
        </header>
        <Button as="link" to="/tools" variant="secondary">
          <span className="btn-icon i-lucide:arrow-left" aria-hidden="true" />
          <span>{t("detail.backToGrid")}</span>
        </Button>
      </div>
    );
  }

  const ToolComponent = tool.Component;

  return (
    <div className="space-y-3">
      <header className="space-y-2">
        <Button as="link" to="/tools" variant="ghost" size="xs">
          <span className="btn-icon i-lucide:arrow-left" aria-hidden="true" />
          <span>{t("detail.backToGrid")}</span>
        </Button>
        <div>
          <h1 className="ui-section-title">{t(tool.titleKey)}</h1>
          <p className="mt-1 text-sm text-text-secondary">{t(tool.descriptionKey)}</p>
        </div>
      </header>
      <ToolComponent />
    </div>
  );
}
