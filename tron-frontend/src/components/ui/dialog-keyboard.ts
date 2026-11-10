import type { KeyboardEvent } from "react";

/** A dialog's single primary footer action is also its plain-Enter action. */
export function handleDialogEnter(event: KeyboardEvent<HTMLElement>) {
  if (event.key !== "Enter" || event.defaultPrevented
    || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229
    || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;

  const dialog = event.currentTarget;
  const target = event.target;
  if (!(target instanceof HTMLElement) || target.closest('[role="dialog"]') !== dialog) return;
  // Keep editing, selection, and focused controls' native keyboard behavior.
  if (target.closest('textarea, [contenteditable]:not([contenteditable="false"]), button, a, select, '
    + '[role="button"], [role="combobox"], [role="listbox"], [role="menu"], [role="tree"], '
    + '[aria-multiline="true"], [aria-expanded="true"], [data-dialog-enter="ignore"]')) return;

  const ownActions = (selector: string) => Array.from(dialog.querySelectorAll<HTMLElement>(selector))
    .filter((action) => action.closest('[role="dialog"]') === dialog && action.getClientRects().length > 0);
  const explicit = ownActions('[data-dialog-primary]');
  const actions = explicit.length ? explicit : ownActions(
    '[data-slot="dialog-footer"] [data-variant="default"], '
    + '[data-slot="dialog-footer"] [data-variant="destructive"], '
    + 'footer [data-variant="default"], footer [data-variant="destructive"], '
    + 'button[type="submit"], input[type="submit"]',
  );
  // Ambiguous multi-action dialogs must explicitly mark their primary action.
  if (actions.length !== 1) return;
  event.preventDefault();
  if (event.repeat) return;
  const action = actions[0];
  if (action.matches(':disabled, [disabled], [aria-disabled="true"], [aria-busy="true"]')) return;
  action.click();
}
