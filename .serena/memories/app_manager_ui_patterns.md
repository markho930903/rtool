# App Manager UI Patterns

- Left panel header area in `src/pages/AppManagerPage.tsx` is kept compact with tighter spacing (`space-y-2`, reduced vertical padding).
- Long helper copy for experimental toggles should prefer tooltip disclosure instead of always-visible description text to reduce visual noise.
- Shared tooltip component is provided at `src/components/ui/tooltip.tsx` and exported via `src/components/ui/index.ts` for reuse in other pages.
- Tooltip trigger should support hover and keyboard focus (`group-hover` + `group-focus-within`) with token-based surface styling.