import { icons as lucideIcons } from "@iconify-json/lucide";
import { icons as notoEmoji } from "@iconify-json/noto";
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

export default defineConfig({
  safelist: [
    "i-lucide:chevrons-left-right",
    "i-lucide:chevrons-up-down",
    "i-noto:desktop-computer",
    "i-noto:hammer-and-wrench",
    "i-noto:outbox-tray",
    "i-noto:scroll",
    "i-noto:card-index-dividers",
    "i-noto:bar-chart",
    "i-noto:satellite-antenna",
    "i-noto:gear",
    "i-noto:clipboard",
    "i-noto:input-symbols",
    "i-noto:magnifying-glass-tilted-right",
    "i-noto:mantelpiece-clock",
  ],
  presets: [
    presetWind4(),
    presetIcons({
      collections: {
        lucide: () => lucideIcons,
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
      "bg-orb-1": "var(--color-bg-orb-1)",
      "bg-orb-2": "var(--color-bg-orb-2)",
      elevated: "var(--color-bg-elevated)",
      surface: "var(--color-surface-card)",
      "surface-card": "var(--color-surface-card)",
      "surface-soft": "var(--color-surface-soft)",
      "surface-glass": "var(--color-surface-glass)",
      "surface-glass-strong": "var(--color-surface-glass-strong)",
      "surface-glass-soft": "var(--color-surface-glass-soft)",
      "surface-overlay": "var(--color-surface-overlay)",
      "surface-popover": "var(--color-surface-popover)",
      "surface-scrim": "var(--color-surface-scrim)",
      "border-muted": "var(--color-border-muted)",
      "border-strong": "var(--color-border-strong)",
      "border-glass": "var(--color-border-glass)",
      "border-glass-strong": "var(--color-border-glass-strong)",
      "text-primary": "var(--color-text-primary)",
      "text-secondary": "var(--color-text-secondary)",
      "text-muted": "var(--color-text-muted)",
      accent: "var(--color-accent)",
      "accent-soft": "var(--color-accent-soft)",
      "accent-contrast": "var(--color-accent-contrast)",
      danger: "var(--color-danger)",
      info: "var(--color-info)",
      "sidebar-item-hover": "var(--color-sidebar-item-hover)",
      "sidebar-item-active": "var(--color-sidebar-item-active)",
      "popover-highlight": "var(--color-popover-highlight)",
      "shimmer-highlight": "var(--color-shimmer-highlight)",
      specular: "var(--color-specular)",
    },
    boxShadow: {
      surface: "var(--shadow-surface)",
      overlay: "var(--shadow-overlay)",
      popover: "var(--shadow-popover)",
      "glass-card": "var(--shadow-glass-card)",
      "glass-elevated": "var(--shadow-glass-elevated)",
      "glass-inset": "var(--shadow-glass-inset)",
      "inset-soft": "var(--shadow-inset-soft)",
      "inset-divider": "var(--shadow-inset-divider)",
      "sidebar-item-active": "var(--shadow-sidebar-item-active)",
      "sidebar-item-hover": "var(--shadow-sidebar-item-hover)",
    },
    borderRadius: {
      2: "var(--radius-2)",
      3: "var(--radius-3)",
      4: "var(--radius-4)",
      sm: "var(--radius-sm)",
      md: "var(--radius-md)",
      lg: "var(--radius-lg)",
      xl: "var(--radius-xl)",
      "2xl": "var(--radius-4)",
      overlay: "var(--radius-overlay)",
    },
    fontSize: {
      "ui-2xs": "var(--font-size-ui-2xs)",
      "ui-xs": "var(--font-size-ui-xs)",
      "ui-sm": "var(--font-size-ui-sm)",
      "ui-md": "var(--font-size-ui-md)",
      "ui-lg": "var(--font-size-ui-lg)",
    },
    lineHeight: {
      "ui-2xs": "var(--line-height-ui-2xs)",
      "ui-xs": "var(--line-height-ui-xs)",
      "ui-sm": "var(--line-height-ui-sm)",
      "ui-md": "var(--line-height-ui-md)",
      "ui-lg": "var(--line-height-ui-lg)",
    },
    spacing: {
      "ui-0-5": "var(--space-0-5)",
      "ui-1": "var(--space-1)",
      "ui-1-5": "var(--space-1-5)",
      "ui-2": "var(--space-2)",
      "ui-2-5": "var(--space-2-5)",
      "ui-3": "var(--space-3)",
      "ui-3-5": "var(--space-3-5)",
      "ui-4": "var(--space-4)",
    },
    letterSpacing: {
      "ui-tight": "var(--letter-spacing-ui-tight)",
      "ui-wide": "var(--letter-spacing-ui-wide)",
      "ui-wider": "var(--letter-spacing-ui-wider)",
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
  border-radius: var(--radius-md);
  overflow: hidden;
}

html[data-window-label="launcher"] #root {
  background: transparent;
  border-radius: var(--radius-md);
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

html[data-window-label="main"] body {
  background: transparent;
  border-radius: var(--radius-md);
  overflow: hidden;
}

html[data-window-label="main"] #root {
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
    "ui-card":
      "relative overflow-hidden rounded-2xl border border-border-glass bg-surface-glass shadow-[var(--shadow-surface)] backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)]",
    "ui-btn-primary":
      "inline-flex items-center gap-1.5 rounded-xl border border-transparent bg-accent px-4 py-2 text-sm font-medium text-accent-contrast transition-opacity hover:opacity-90",
    "ui-btn-secondary":
      "inline-flex items-center gap-1.5 rounded-xl border border-border-glass bg-surface-glass-soft px-4 py-2 text-sm text-text-primary shadow-[var(--shadow-inset-soft)] transition-colors hover:border-border-glass-strong hover:bg-surface-glass",
    "ui-section-title": "text-xl font-semibold text-text-primary",
    "ui-glass-panel":
      "relative overflow-hidden rounded-xl border border-border-glass bg-surface-glass shadow-[var(--shadow-surface)] backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)]",
    "ui-glass-panel-strong":
      "relative overflow-hidden rounded-xl border border-border-glass-strong bg-surface-glass-strong shadow-[var(--shadow-overlay)] backdrop-blur-[var(--glass-blur)] backdrop-saturate-[var(--glass-saturate)]",
    "ui-glass-chip":
      "inline-flex items-center rounded-full border border-border-glass bg-surface-glass-soft px-2 py-0.5 shadow-[var(--shadow-inset-soft)]",
    "ui-glass-hover":
      "transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out hover:border-border-glass-strong hover:bg-surface-glass-soft hover:shadow-[var(--shadow-inset-soft)]",
    "text-ui-2xs": "[font-size:var(--font-size-ui-2xs)]",
    "text-ui-xs": "[font-size:var(--font-size-ui-xs)]",
    "text-ui-sm": "[font-size:var(--font-size-ui-sm)]",
    "text-ui-md": "[font-size:var(--font-size-ui-md)]",
    "text-ui-lg": "[font-size:var(--font-size-ui-lg)]",
    "leading-ui-2xs": "[line-height:var(--line-height-ui-2xs)]",
    "leading-ui-xs": "[line-height:var(--line-height-ui-xs)]",
    "leading-ui-sm": "[line-height:var(--line-height-ui-sm)]",
    "leading-ui-md": "[line-height:var(--line-height-ui-md)]",
    "leading-ui-lg": "[line-height:var(--line-height-ui-lg)]",
    "ui-text-micro": "text-ui-2xs leading-ui-2xs",
    "ui-text-caption": "text-ui-xs leading-ui-xs",
    "ui-text-body-sm": "text-ui-sm leading-ui-sm",
    "ui-text-body": "text-ui-md leading-ui-md",
    "ui-badge":
      "rounded-full border border-border-muted bg-surface px-2 py-0.5 text-ui-2xs leading-ui-2xs tracking-ui-wide",

    // layer: project-wide helper shortcuts
    "btn-icon": "inline-block h-[1.05em] w-[1.05em] shrink-0 align-[-0.14em]",
  },
});
