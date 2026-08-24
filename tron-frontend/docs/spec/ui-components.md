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
