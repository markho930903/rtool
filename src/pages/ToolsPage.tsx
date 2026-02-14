import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";

import Base64Tool from "@/components/tools/Base64Tool";
import RegexTool from "@/components/tools/RegexTool";
import TimestampTool from "@/components/tools/TimestampTool";

const TOOL_ITEMS = [
  { id: "base64", element: <Base64Tool /> },
  { id: "timestamp", element: <TimestampTool /> },
  { id: "regex", element: <RegexTool /> },
] as const;

export default function ToolsPage() {
  const { t } = useTranslation("tools");
  const [searchParams] = useSearchParams();
  const activeToolId = (searchParams.get("tool") ?? "").trim().toLowerCase();

  useEffect(() => {
    if (!activeToolId) {
      return;
    }

    const element = document.getElementById(`tool-card-${activeToolId}`);
    if (!element) {
      return;
    }

    element.scrollIntoView({ behavior: "smooth", block: "center" });
    element.classList.add("ring-2", "ring-accent");

    const timer = window.setTimeout(() => {
      element.classList.remove("ring-2", "ring-accent");
    }, 1200);

    return () => {
      window.clearTimeout(timer);
      element.classList.remove("ring-2", "ring-accent");
    };
  }, [activeToolId]);

  return (
    <div className="space-y-3">
      <header>
        <h1 className="ui-section-title">{t("page.title")}</h1>
        <p className="mt-1 text-sm text-text-secondary">{t("page.subtitle")}</p>
      </header>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-3">
        {TOOL_ITEMS.map((item) => (
          <div key={item.id} id={`tool-card-${item.id}`} className="rounded-lg transition-shadow duration-200">
            {item.element}
          </div>
        ))}
      </div>
    </div>
  );
}
