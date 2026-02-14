import {
  defineConfig,
  presetAttributify,
  presetIcons,
  presetTagify,
  presetTypography,
  presetWind4,
  transformerAttributifyJsx,
  transformerCompileClass,
  transformerDirectives,
  transformerVariantGroup,
} from "unocss";

import { icons as notoEmoji } from "@iconify-json/noto";

export default defineConfig({
  presets: [
    presetWind4(),
    presetIcons({
      collections: {
        noto: () => notoEmoji,
      },
    }),
    presetTagify(),
    presetTypography(),
    presetAttributify(),
  ],
  transformers: [
    transformerDirectives(),
    transformerVariantGroup(),
    transformerAttributifyJsx(),
    transformerCompileClass(),
  ],
  theme: {
    colors: {
      app: "var(--color-bg-app)",
      elevated: "var(--color-bg-elevated)",
      surface: "var(--color-surface-card)",
      "surface-soft": "var(--color-surface-soft)",
      "surface-overlay": "var(--color-surface-overlay)",
      "surface-scrim": "var(--color-surface-scrim)",
      "border-muted": "var(--color-border-muted)",
      "border-strong": "var(--color-border-strong)",
      "text-primary": "var(--color-text-primary)",
      "text-secondary": "var(--color-text-secondary)",
      "text-muted": "var(--color-text-muted)",
      accent: "var(--color-accent)",
      "accent-soft": "var(--color-accent-soft)",
      "accent-contrast": "var(--color-accent-contrast)",
      danger: "var(--color-danger)",
      info: "var(--color-info)",
    },
    borderRadius: {
      sm: "var(--radius-sm)",
      md: "var(--radius-md)",
      lg: "var(--radius-lg)",
      xl: "var(--radius-xl)",
    },
  },
  preflights: [
    {
      getCSS: () => `
html,
body,
#root {
  height: 100%;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: var(--color-bg-app);
  color: var(--color-text-primary);
  transition:
    background-color 180ms ease,
    color 180ms ease;
}

html[data-window-label="launcher"] body {
  background: transparent;
  border-radius: var(--radius-overlay);
  overflow: hidden;
}

html[data-window-label="launcher"] #root {
  background: transparent;
  border-radius: var(--radius-overlay);
  overflow: hidden;
}

html[data-window-label="clipboard_history"] body {
  background: transparent;
  border-radius: var(--radius-md);
  overflow: hidden;
}

html[data-window-label="clipboard_history"] #root {
  background: transparent;
  border-radius: var(--radius-md);
  overflow: hidden;
}

a {
  color: inherit;
}

button,
input,
select,
textarea {
  font: inherit;
  color: inherit;
}

::selection {
  background: var(--color-accent-soft);
  color: var(--color-text-primary);
}
`,
    },
  ],
  shortcuts: {
    // layer: global reusable ui primitives only
    // 约束：shortcuts 仅用于跨页面复用的通用样式与设计规范，不承载页面/模块私有布局样式。
    "ui-page": "min-h-screen bg-app text-text-primary",
    "ui-card": "rounded-2xl border border-border-muted bg-surface",
    "ui-btn-primary":
      "inline-flex items-center gap-1.5 rounded-xl border border-transparent bg-accent px-4 py-2 text-sm font-medium text-accent-contrast transition-opacity hover:opacity-90",
    "ui-btn-secondary":
      "inline-flex items-center gap-1.5 rounded-xl border border-border-strong bg-surface px-4 py-2 text-sm text-text-primary transition-colors hover:bg-surface-soft",
    "ui-section-title": "text-xl font-semibold text-text-primary",

    // layer: project-wide helper shortcuts
    "btn-icon": "inline-block h-[1.05em] w-[1.05em] shrink-0 align-[-0.14em]",
  },
});
