import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PACKAGE_PATH = path.join(ROOT, "node_modules/@xterm/xterm/package.json");
const BUNDLE_PATH = path.join(ROOT, "node_modules/@xterm/xterm/lib/xterm.js");
const VERSION = "5.5.0";

const ORIGINAL_INPUT_EVENT =
  '_inputEvent(e){if(e.data&&"insertText"===e.inputType&&(!e.composed||!this._keyDownSeen)&&!this.optionsService.rawOptions.screenReaderMode){if(this._keyPressHandled)return!1;this._unprocessedDeadKey=!1;const t=e.data;return this.coreService.triggerDataEvent(t,!0),this.cancel(e),!0}return!1}';
const PATCHED_INPUT_EVENT =
  '_inputEvent(e){if(e.data&&"insertText"===e.inputType&&(!e.composed||!this._keyDownSeen||!this._compositionHelper.isComposing&&!this._compositionHelper.isSendingComposition)&&!this.optionsService.rawOptions.screenReaderMode){if(this._keyPressHandled)return!1;this._unprocessedDeadKey=!1;const t=e.data;return this.coreService.triggerDataEvent(t,!0),this._compositionHelper.inputHandled(t),this.cancel(e),!0}return!1}';
const ORIGINAL_COMPOSITION =
  'compositionend(){this._finalizeComposition(!0)}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode||(this._handleAnyTextareaChanges(),!1)}';
const PATCHED_COMPOSITION =
  'compositionend(){this._finalizeComposition(!0)}get isSendingComposition(){return this._isSendingComposition}inputHandled(e){this._isComposing||this._isSendingComposition||(this._dataAlreadySent=e)}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode||(this._dataAlreadySent?(this._dataAlreadySent="",!1):(this._handleAnyTextareaChanges(),!1))}';

function replaceExactly(bundle, original, patched, name) {
  if (bundle.includes(patched)) {
    return bundle;
  }
  const first = bundle.indexOf(original);
  if (first === -1 || bundle.indexOf(original, first + 1) !== -1) {
    throw new Error(`xterm ${VERSION} ${name} signature changed; refusing an unsafe IME patch`);
  }
  return `${bundle.slice(0, first)}${patched}${bundle.slice(first + original.length)}`;
}

function patchXtermImeBundle(bundle) {
  return replaceExactly(
    replaceExactly(bundle, ORIGINAL_INPUT_EVENT, PATCHED_INPUT_EVENT, "input event"),
    ORIGINAL_COMPOSITION,
    PATCHED_COMPOSITION,
    "composition helper",
  );
}

function patchInstalledBundle() {
  const pkg = JSON.parse(fs.readFileSync(PACKAGE_PATH, "utf8"));
  if (pkg.version !== VERSION) {
    throw new Error(`xterm IME patch requires ${VERSION}, found ${pkg.version}`);
  }
  const original = fs.readFileSync(BUNDLE_PATH, "utf8");
  const patched = patchXtermImeBundle(original);
  if (patched !== original) {
    fs.writeFileSync(BUNDLE_PATH, patched);
  }
}

patchInstalledBundle();

export { patchXtermImeBundle };
