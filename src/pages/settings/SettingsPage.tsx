import SettingsSidebar from "@/pages/settings/components/SettingsSidebar";
import ClipboardSettingsSection from "@/pages/settings/components/ClipboardSettingsSection";
import GeneralSettingsSection from "@/pages/settings/components/GeneralSettingsSection";
import LauncherSettingsSection from "@/pages/settings/components/LauncherSettingsSection";
import LoggingSettingsSection from "@/pages/settings/components/LoggingSettingsSection";
import TransferSettingsSection from "@/pages/settings/components/TransferSettingsSection";
import { useSettingsPageState } from "@/pages/settings/hooks/useSettingsPageState";

export default function SettingsPage() {
  const state = useSettingsPageState();

  return (
    <div className="h-full min-h-0">
      <div className="grid h-full min-h-0 grid-cols-1 md:grid-cols-[220px_1fr]">
        <SettingsSidebar
          items={state.nav.settingsNavItems}
          activeSection={state.nav.activeSection}
          onSectionChange={state.nav.setActiveSection}
        />

        <div className="min-h-0 overflow-y-auto p-4">
          {state.nav.activeSection === "general" ? <GeneralSettingsSection state={state.general} /> : null}
          {state.nav.activeSection === "clipboard" ? <ClipboardSettingsSection state={state.clipboard} /> : null}
          {state.nav.activeSection === "transfer" ? <TransferSettingsSection state={state.transfer} /> : null}
          {state.nav.activeSection === "launcher" ? <LauncherSettingsSection state={state.launcher} /> : null}
          {state.nav.activeSection === "logging" ? <LoggingSettingsSection state={state.logging} /> : null}
        </div>
      </div>
    </div>
  );
}
