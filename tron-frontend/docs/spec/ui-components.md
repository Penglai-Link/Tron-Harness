# Shared UI Components

This document catalogs the reusable primitives in `src/components/ui`. It is the first reference when implementing a page or domain component.

## Usage Rules

- Check this catalog before creating a reusable control.
- Prefer an existing primitive and compose it through its public exports.
- Use `className` and documented props for contextual styling. Keep business rules and data behavior in domain components.
- Use translation keys for user-facing labels, descriptions, errors, and status text. The primitives must remain locale-agnostic.
- Preserve keyboard navigation, focus visibility, semantic HTML, and the accessibility behavior provided by the underlying primitive.
- Add a domain component when behavior is domain-specific. Add a new shared primitive only when no existing entry supports the interaction.

## Component Catalog

| File | Primary exports | Use for |
| --- | --- | --- |
| `badge.tsx` | `Badge`, `badgeVariants`, `BadgeProps` | Compact status, category, and metadata labels |
| `button.tsx` | `Button`, `buttonVariants` | Primary, secondary, destructive, and icon actions |
| `card.tsx` | `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter` | Framed content and grouped information |
| `combobox.tsx` | `Combobox`, `ComboboxInput`, `ComboboxTrigger`, `ComboboxPopup`, `ComboboxItem`, `ComboboxValue`, `ComboboxList`, `ComboboxClear`, `ComboboxEmpty`, `ComboboxGroup`, `ComboboxGroupLabel`, `ComboboxSeparator`, `ComboboxRow`, `ComboboxStatus`, `ComboboxCollection`, `ComboboxChips`, `ComboboxChip`, `ComboboxChipRemove`, `ComboboxChipsInput`, `useComboboxFilter` | Searchable single- or multi-value selection |
| `context-menu.tsx` | `ContextMenu`, `ContextMenuTrigger`, `ContextMenuContent`, `ContextMenuItem`, `ContextMenuSeparator` | Pointer-positioned contextual actions; trigger must expose a semantic target and item labels come from the consumer's translations |
| `dialog.tsx` | `Dialog`, `DialogTrigger`, `DialogPortal`, `DialogClose`, `DialogOverlay`, `DialogContent`, `DialogHeader`, `DialogFooter`, `DialogTitle`, `DialogDescription` | Modal workflows and focused confirmations |
| `dropdown-menu.tsx` | `DropdownMenu`, `DropdownMenuTrigger`, `DropdownMenuContent`, `DropdownMenuGroup`, `DropdownMenuLabel`, `DropdownMenuItem`, `DropdownMenuSub`, `DropdownMenuSubTrigger`, `DropdownMenuSubContent`, `DropdownMenuCheckboxItem`, `DropdownMenuRadioGroup`, `DropdownMenuRadioItem`, `DropdownMenuSeparator`, `DropdownMenuShortcut` | Contextual actions and option menus |
| `field.tsx` | `Field`, `FieldLabel`, `FieldItem`, `FieldDescription`, `FieldError`, `FieldControl`, `FieldValidity`, `FieldPrimitive` | Form field layout, descriptions, and validation messages |
| `input.tsx` | `Input`, `InputPrimitive`, `InputProps` | Single-line text and value entry |
| `label.tsx` | `Label` | Accessible labels for controls |
| `resizable.tsx` | `ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle` | User-adjustable panel layouts |
| `scroll-area.tsx` | `ScrollArea`, `ScrollBar`, `ScrollAreaPrimitive` | Scrollable content with controlled scrollbars |
| `sidebar.tsx` | `SidebarProvider`, `Sidebar`, `SidebarContent`, `SidebarFooter`, `SidebarGroup`, `SidebarMenu`, `SidebarMenuItem`, `SidebarMenuButton`, `SidebarMenuSub`, `useSidebar` | Application sidebar layouts with controlled desktop collapse and a keyboard-accessible fixed mobile drawer; consumers provide translated labels and remain responsible for navigation behavior |
| `select.tsx` | `Select`, `SelectButton`, `SelectTrigger`, `SelectValue`, `SelectPopup`, `SelectItem`, `SelectSeparator`, `SelectGroup`, `SelectLabel`, `SelectGroupLabel`, `SelectPrimitive`, `selectTriggerVariants`, `selectTriggerIconClassName`, `SelectButtonProps` | Structured single-value selection |
| `spinner.tsx` | `Spinner` | Loading and pending states |
| `switch.tsx` | `Switch`, `SwitchPrimitive` | Immediate binary settings |
| `table.tsx` | `Table`, `TableHeader`, `TableBody`, `TableFooter`, `TableRow`, `TableHead`, `TableCell`, `TableCaption` | Semantic tabular data |
| `tabs.tsx` | `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`, `tabsListVariants` | Peer views within the same route or context |
| `textarea.tsx` | `Textarea` | Multi-line text entry |
| `toast.tsx` | `ToastProvider`, `AnchoredToastProvider`, `toastManager`, `anchoredToastManager`, `ToastPosition`, `ToastProviderProps`, `AnchoredToastProviderProps`, `ToastPrimitive` | Transient feedback and action results |
| `tooltip.tsx` | `Tooltip`, `TooltipProvider`, `TooltipTrigger`, `TooltipPopup`, `TooltipCreateHandle`, `TooltipPrimitive` | Short contextual help for unfamiliar controls |

## Usage Example

```tsx
import { Button } from "@/components/ui/button"
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"

function ProjectNameField() {
  return (
    <Field>
      <FieldLabel>{t("projects.name")}</FieldLabel>
      <Input aria-describedby="project-name-help" />
      <FieldDescription id="project-name-help">
        {t("projects.nameHelp")}
      </FieldDescription>
      <Button type="submit">{t("common.save")}</Button>
    </Field>
  )
}
```

The example shows composition only. Translation functions, validation state, and submit behavior belong to the consuming domain component.

## Adding or Changing a Primitive

When a primitive is added or materially changed:

1. Confirm that the existing catalog does not already cover the interaction.
2. Keep the API generic and strongly typed. Expose only intentional public exports.
3. Preserve semantic markup, keyboard operation, focus behavior, and screen-reader state.
4. Keep user-facing text outside the primitive or resolve it through translation keys.
5. Add focused tests for the component's behavior and important accessibility states.
6. Update this catalog in the same change with the file, exports, purpose, usage guidance, accessibility notes, internationalization considerations, and tests.
7. Run the frontend checks through the repository Makefile.

## Maintenance Checklist

- The listed file exists under `src/components/ui`.
- The primary exports match the source file.
- The component remains generic and domain-agnostic.
- Keyboard and focus behavior remain usable.
- User-facing text remains translatable.
- Focused tests cover the changed behavior.

Theme packages inherit public component tokens from `src/shared/theme/components.css` and primitive utility classes:

- Button: six variants (`primary` maps to `default`), independent normal/hover/active foreground and background, border, disabled state, focus outline, radius, dimensions and typography via `--tron-button-*`.
- Input/Textarea and Forms wrappers: `--tron-input-*` controls normal, hover, focus, disabled and invalid paint, radius and density. Select has `--tron-select-*` paint/radius with input fallbacks.
- Table and both DataTable wrappers: `--tron-table-*` controls normal/striped/hover/selected rows, header, row height, cell padding, text and dividers. Selection/expansion wins over hover. Sticky cells must use `var(--_tron-row-bg)` for body and `var(--tron-table-header-bg, var(--color-elevated-surface))` for headers.
- SurfaceCard/MetricCard: `--tron-card-*` colors/radius; SurfaceCard also exposes padding and shadow. Dialog/Sheet, dropdown/select popups and overlays expose their own paint tokens.
- ClassName/style remain explicit caller overrides. For a single button/row, set the same public CSS variables on that element; do not reset these inherited public variables in a descendant selector.
- Preserve each control's accessible name, keyboard events and disabled semantics when changing styles. Changing theme must not change business behavior; verify visual states in the actual pages that use these controls.

The complete author-facing token contract is in `tron-skills/tron-theme-spec/references/tokens.md` at repository root. New public tokens must update that contract and `tokens.json`. Browser acceptance checks computed button states, input focus, selected/striped rows, pinned backgrounds, portal fonts, and cleanup when returning to a builtin. Runtime regression tests live in `src/shared/theme`.
