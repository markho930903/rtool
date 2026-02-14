import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui";

export default function NotFoundPage() {
  const { t } = useTranslation("notFound");

  return (
    <div className="space-y-3">
      <h1 className="ui-section-title">{t("title")}</h1>
      <p className="text-sm text-text-secondary">{t("description")}</p>
      <Button as="link" to="/" variant="secondary">
        <span className="btn-icon i-noto:left-arrow" aria-hidden="true" />
        <span>{t("backHome")}</span>
      </Button>
    </div>
  );
}
