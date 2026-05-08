---
name: Tron
colors:
  surface: '#fdf8f8'
  surface-dim: '#ddd9d8'
  surface-bright: '#fdf8f8'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f7f3f2'
  surface-container: '#f1edec'
  surface-container-high: '#ebe7e6'
  surface-container-highest: '#e5e2e1'
  on-surface: '#1c1b1b'
  on-surface-variant: '#444748'
  inverse-surface: '#313030'
  inverse-on-surface: '#f4f0ef'
  outline: '#747878'
  outline-variant: '#c4c7c7'
  surface-tint: '#5f5e5e'
  primary: '#000000'
  on-primary: '#ffffff'
  primary-container: '#1c1b1b'
  on-primary-container: '#858383'
  accent: '#2f5f9f'
  on-accent: '#ffffff'
  accent-container: '#dbe7f7'
  on-accent-container: '#133761'
  focus-ring: '#2f5f9f'
  inverse-primary: '#c8c6c5'
  secondary: '#5e5e63'
  on-secondary: '#ffffff'
  secondary-container: '#e0dfe4'
  on-secondary-container: '#626267'
  tertiary: '#000000'
  on-tertiary: '#ffffff'
  tertiary-container: '#1c1b1a'
  on-tertiary-container: '#868382'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  success: '#4f6f52'
  on-success: '#ffffff'
  success-container: '#e4efe5'
  on-success-container: '#253927'
  warning: '#8a6a1f'
  on-warning: '#ffffff'
  warning-container: '#f4e8c8'
  on-warning-container: '#3f2f0b'
  row-stripe: '#f9f9fb'
  hover-surface: '#f2f2f7'
  card-border: '#e5e5e5'
  primary-fixed: '#e5e2e1'
  primary-fixed-dim: '#c8c6c5'
  on-primary-fixed: '#1c1b1b'
  on-primary-fixed-variant: '#474746'
  secondary-fixed: '#e3e2e7'
  secondary-fixed-dim: '#c7c6cb'
  on-secondary-fixed: '#1a1b1f'
  on-secondary-fixed-variant: '#46464b'
  tertiary-fixed: '#e6e2df'
  tertiary-fixed-dim: '#cac6c4'
  on-tertiary-fixed: '#1c1b1a'
  on-tertiary-fixed-variant: '#484645'
  background: '#fdf8f8'
  on-background: '#1c1b1b'
  surface-variant: '#e5e2e1'
typography:
  display:
    fontFamily: Inter
    fontSize: 40px
    fontWeight: '600'
    lineHeight: '1.1'
    letterSpacing: '0'
  h1:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: '1.3'
    letterSpacing: '0'
  h2:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: '600'
    lineHeight: '1.3'
    letterSpacing: '0'
  body-lg:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '400'
    lineHeight: '1.5'
    letterSpacing: '0'
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: '1.5'
    letterSpacing: '0'
  body-sm:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: '1.4'
    letterSpacing: '0'
  label-caps:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: '600'
    lineHeight: '1'
    letterSpacing: 0.05em
  tabular:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: '1.4'
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  base: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 40px
  gutter: 16px
  margin: 24px
motion:
  fast: 120ms
  standard: 180ms
  slow: 240ms
  easing: cubic-bezier(0.2, 0, 0, 1)
breakpoints:
  sm: 640px
  md: 768px
  lg: 960px
  xl: 1200px
zIndex:
  base: 0
  sticky: 20
  overlay: 40
  modal: 60
  toast: 80
---

## Brand & Style

The design system is rooted in **Apple-like minimalism**, prioritizing clarity, precision, and an unobtrusive aesthetic. It is engineered for the modern enterprise SaaS environment, specifically tailored for Agent Workspaces where high information density must coexist with cognitive ease.

The brand personality is **composed, efficient, and sophisticated**. It avoids the "playful" tropes of consumer tech in favor of a "tool-first" mentality. By utilizing expansive whitespace, refined typography, and a monochromatic foundation, the system ensures that the agent's focus remains entirely on the data and the task at hand. The emotional response is one of calm control in a fast-paced work environment.

## Colors

The palette is strictly curated to eliminate visual noise. The foundation is built on **Apple’s signature off-whites and cool grays**, ensuring the interface feels premium and lightweight.

- **Primary & Secondary:** We use near-blacks (#1C1B1B) for maximum legibility in text and deep grays (#444748) for secondary metadata.
- **Accents:** Accent colors are used sparingly. The primary blue is #2F5F9F, a low-saturation professional hue used only for focus rings, critical actions, and active states.
- **Functional Grays:** A spectrum of light grays defines boundaries without the harshness of solid lines.
- **Semantic Colors:** Success uses #4F6F52, Warning uses #8A6A1F, and Error uses #BA1A1A. Their container colors are intentionally pale so dense tables remain calm.

## Typography

This design system employs **Inter** for its exceptional readability and neutral, systematic character. The typographic scale is optimized for information density; while headings are bold and clear, the "Body-SM" and "Tabular" levels are the workhorses of the agent workspace.

To maintain an organized feel in dense data views, use **Tabular Figures** for all numerical data. Letter spacing remains non-negative across the runtime UI; large headlines use 0 tracking, while small labels use 0.05em for legibility.

## Layout & Spacing

The layout philosophy follows a **Fluid Grid with strict 4px/8px increments**. This "soft grid" ensures consistency while allowing the workspace to adapt to the multi-pane nature of agent dashboards.

- **Agent Sidebar:** Fixed at 280px to provide a stable anchor.
- **Main Canvas:** Fluid, utilizing a 12-column system with 16px gutters.
- **Density:** High density is achieved not by shrinking text, but by reducing vertical padding in lists (8px) and increasing horizontal margins (24px) to create "breathing room" at the edges of the screen.

## Elevation & Depth

Depth is conveyed through **Tonal Layering and Soft Ambient Shadows**, avoiding heavy borders which can clutter a dense UI.

- **Level 0 (Background):** #FDF8F8. The base canvas.
- **Level 1 (Cards/Surface):** #FFFFFF. Used for the primary workspace containers.
- **Shadows:** We use a "Natural Light" shadow model. Shadows are highly diffused, with a very low opacity (4-8% Alpha) and no color tinting. They should appear as a subtle lift rather than a distinct drop shadow.
- **Interactions:** Hover states should utilize a subtle 1px inner stroke or a slight shift in background gray (#F2F2F7) rather than a heavy shadow increase. Motion uses 120ms fast, 180ms standard, and 240ms slow transitions with cubic-bezier(0.2, 0, 0, 1).

## Responsive & Layering

- **Breakpoints:** sm 640px, md 768px, lg 960px, xl 1200px.
- **Z-Index:** base 0, sticky 20, overlay 40, modal 60, toast 80.

## Shapes

The design system utilizes **Rounded** geometry (Level 2). This provides a modern, approachable feel that softens the "industrial" nature of enterprise data.

- **Standard Elements (Buttons/Inputs):** 0.5rem (8px).
- **Cards & Containers:** 1rem (16px).
- **Outer Wrappers:** 1.5rem (24px).

Large radius values are reserved for the primary containers of the workspace, creating a "nested" look that clearly defines different functional areas (e.g., Chat Pane vs. CRM Details).

## Components

Components focus on utility and "quiet" interaction.

- **Buttons:** Primary buttons are solid dark gray/black with white text. Secondary buttons use a light gray ghost style with no border, becoming visible only on hover.
- **Breadcrumbs:** Breadcrumb navigation is conditionally displayed only on child-level pages within a primary module. Top-level module pages must not show breadcrumbs. Nested pages such as create, edit, detail, or settings views should show a breadcrumb above the page heading to indicate the relationship between the parent module and the current page. Parent breadcrumb items are interactive links, while the current page item is rendered as non-interactive text.
- **Cards:** Rounded cards are the primary container. They use a subtle 1px border (#E5E5E5) and a soft level-1 shadow.
- **Input Fields:** Minimalist design with a 1px light gray border. Focus states use a subtle 2px blue "glow" or a thicker 2px black border.
- **Chips/Badges:** Small, pill-shaped, and low-contrast. Backgrounds are very light tints of the semantic color with dark text.
- **Lists:** High-density lists with 8px vertical padding. Use "Zebra striping" with #F9F9FB for rows only when data exceeds 10 columns.
- **Data Tables:** Borderless rows with a 1px bottom separator. Headers use the "Label-Caps" typography style for clear structural hierarchy.
- **Agent Feed:** Message bubbles use a soft-gray background for the system/customer and white with a subtle shadow for the agent's own responses.
