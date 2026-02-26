import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui";
import type { SettingsNavItem, SettingsSection } from "@/pages/settings/hooks/useSettingsPageState";

interface SettingsSidebarProps {
  items: SettingsNavItem[];
  activeSection: SettingsSection;
  onSectionChange: (section: SettingsSection) => void;
}

export default function SettingsSidebar(props: SettingsSidebarProps) {
  const { t } = useTranslation("settings");

  return (
    <aside className="ui-glass-panel h-full min-h-0 border-b border-border-glass bg-surface-glass md:border-b-0 md:border-r">
      <nav className="flex h-full flex-col py-5" aria-label={t("nav.aria")}>
        {props.items.map((item) => {
          const active = item.key === props.activeSection;
          return (
            <Button
              unstyled
              key={item.key}
              type="button"
              className={[
                "w-full border-b border-border-glass px-4 py-3 text-left transition-colors last:border-b-0",
                active
                  ? "bg-accent-soft text-text-primary"
                  : "text-text-secondary hover:bg-surface-glass-soft hover:text-text-primary",
              ].join(" ")}
              onClick={() => props.onSectionChange(item.key)}
              aria-current={active ? "page" : undefined}
            >
              <div className="flex items-start gap-2.5">
                <span
                  className={`settings-nav-icon btn-icon mt-0.5 shrink-0 text-[1rem] ${item.icon}`}
                  aria-hidden="true"
                />
                <div className="min-w-0">
                  <div className="text-sm font-semibold">{item.label}</div>
                  <div className="mt-0.5 text-xs text-text-muted">{item.description}</div>
                </div>
              </div>
            </Button>
          );
        })}
      </nav>
    </aside>
  );
}
