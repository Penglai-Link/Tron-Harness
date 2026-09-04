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
  '_inputEvent(e){if(e.data&&"insertText"===e.inputType&&!this._compositionHelper.isComposing&&!this.optionsService.rawOptions.screenReaderMode){if(this._keyPressHandled)return!1;this._unprocessedDeadKey=!1;const t=e.data;return this._compositionHelper.inputHandled(t),this.coreService.triggerDataEvent(t,!0),this.cancel(e),!0}return!1}';
const PREVIOUS_PATCHED_INPUT_EVENT =
  '_inputEvent(e){if(e.data&&"insertText"===e.inputType&&!this._compositionHelper.isComposing&&!this.optionsService.rawOptions.screenReaderMode){if(this._keyPressHandled)return!1;this._unprocessedDeadKey=!1;const t=e.data;return this.coreService.triggerDataEvent(t,!0),this._compositionHelper.inputHandled(),this.cancel(e),!0}return!1}';
const LEGACY_PATCHED_INPUT_EVENT =
  '_inputEvent(e){if(e.data&&"insertText"===e.inputType&&(!e.composed||!this._keyDownSeen||!this._compositionHelper.isComposing&&!this._compositionHelper.isSendingComposition)&&!this.optionsService.rawOptions.screenReaderMode){if(this._keyPressHandled)return!1;this._unprocessedDeadKey=!1;const t=e.data;return this.coreService.triggerDataEvent(t,!0),this._compositionHelper.inputHandled(t),this.cancel(e),!0}return!1}';
const ORIGINAL_COMPOSITION =
  'compositionstart(){this._isComposing=!0,this._compositionPosition.start=this._textarea.value.length,this._compositionView.textContent="",this._dataAlreadySent="",this._compositionView.classList.add("active")}compositionupdate(e){this._compositionView.textContent=e.data,this.updateCompositionElements(),setTimeout((()=>{this._compositionPosition.end=this._textarea.value.length}),0)}compositionend(){this._finalizeComposition(!0)}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode||(this._handleAnyTextareaChanges(),!1)}';
const PATCHED_COMPOSITION =
  'compositionstart(){this._isComposing=!0,this._isSendingComposition=!1,this._pendingCompositionData="",this._compositionPosition.start=this._textarea.value.length,this._compositionView.textContent="",this._compositionView.classList.add("active")}compositionupdate(e){this._compositionView.textContent=e.data,this.updateCompositionElements(),setTimeout((()=>{this._compositionPosition.end=this._textarea.value.length}),0)}compositionend(e){this._finalizeComposition(!0,e.data)}inputHandled(e){if(!this._isSendingComposition)return;const t=this._pendingCompositionData;this._isSendingComposition=!1,this._pendingCompositionData="",t&&t!==e&&this._coreService.triggerDataEvent(t,!0)}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode}';
const PREVIOUS_PATCHED_COMPOSITION =
  'compositionstart(){this._isComposing=!0,this._isSendingComposition=!1,this._compositionPosition.start=this._textarea.value.length,this._compositionView.textContent="",this._compositionView.classList.add("active")}compositionupdate(e){this._compositionView.textContent=e.data,this.updateCompositionElements(),setTimeout((()=>{this._compositionPosition.end=this._textarea.value.length}),0)}compositionend(){this._finalizeComposition(!0)}inputHandled(){this._isSendingComposition=!1}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode}';
const LEGACY_PATCHED_COMPOSITION =
  'compositionstart(){this._isComposing=!0,this._compositionPosition.start=this._textarea.value.length,this._compositionView.textContent="",this._dataAlreadySent="",this._compositionView.classList.add("active")}compositionupdate(e){this._compositionView.textContent=e.data,this.updateCompositionElements(),setTimeout((()=>{this._compositionPosition.end=this._textarea.value.length}),0)}compositionend(){this._finalizeComposition(!0)}get isSendingComposition(){return this._isSendingComposition}inputHandled(e){this._isComposing||this._isSendingComposition||(this._dataAlreadySent=e)}keydown(e){if(this._isComposing||this._isSendingComposition){if(229===e.keyCode)return!1;if(16===e.keyCode||17===e.keyCode||18===e.keyCode)return!1;this._finalizeComposition(!1)}return 229!==e.keyCode||(this._dataAlreadySent?(this._dataAlreadySent="",!1):(this._handleAnyTextareaChanges(),!1))}';
const ORIGINAL_FINALIZE_AND_DIFF =
  '_finalizeComposition(e){if(this._compositionView.classList.remove("active"),this._isComposing=!1,e){const e={start:this._compositionPosition.start,end:this._compositionPosition.end};this._isSendingComposition=!0,setTimeout((()=>{if(this._isSendingComposition){let t;this._isSendingComposition=!1,e.start+=this._dataAlreadySent.length,t=this._isComposing?this._textarea.value.substring(e.start,e.end):this._textarea.value.substring(e.start),t.length>0&&this._coreService.triggerDataEvent(t,!0)}}),0)}else{this._isSendingComposition=!1;const e=this._textarea.value.substring(this._compositionPosition.start,this._compositionPosition.end);this._coreService.triggerDataEvent(e,!0)}}_handleAnyTextareaChanges(){const e=this._textarea.value;setTimeout((()=>{if(!this._isComposing){const t=this._textarea.value,i=t.replace(e,"");this._dataAlreadySent=i,t.length>e.length?this._coreService.triggerDataEvent(i,!0):t.length<e.length?this._coreService.triggerDataEvent(`${a.C0.DEL}`,!0):t.length===e.length&&t!==e&&this._coreService.triggerDataEvent(t,!0)}}),0)}';
const PATCHED_FINALIZE =
  '_finalizeComposition(e,t=""){if(this._compositionView.classList.remove("active"),this._isComposing=!1,e){this._pendingCompositionData=t||this._textarea.value.substring(this._compositionPosition.start),this._isSendingComposition=!0,setTimeout((()=>{if(!this._isSendingComposition)return;const e=this._pendingCompositionData;this._isSendingComposition=!1,this._pendingCompositionData="",e&&this._coreService.triggerDataEvent(e,!0)}),0)}else{this._isSendingComposition=!1;const e=this._pendingCompositionData||this._textarea.value.substring(this._compositionPosition.start,this._compositionPosition.end);this._pendingCompositionData="",this._coreService.triggerDataEvent(e,!0)}}';
const PREVIOUS_PATCHED_FINALIZE =
  '_finalizeComposition(e){if(this._compositionView.classList.remove("active"),this._isComposing=!1,e)this._isSendingComposition=!0;else{this._isSendingComposition=!1;const e=this._textarea.value.substring(this._compositionPosition.start,this._compositionPosition.end);this._coreService.triggerDataEvent(e,!0)}}';
const ORIGINAL_COMPOSITION_END_LISTENER =
  '"compositionend",(()=>this._compositionHelper.compositionend())';
const PATCHED_COMPOSITION_END_LISTENER =
  '"compositionend",(e=>this._compositionHelper.compositionend(e))';

function replaceExactly(bundle, original, patched, name, legacy = []) {
  if (bundle.includes(patched)) {
    return bundle;
  }
  const candidate = [original, ...legacy].find((value) => {
    const first = bundle.indexOf(value);
    return first !== -1 && bundle.indexOf(value, first + 1) === -1;
  });
  if (!candidate) {
    throw new Error(`xterm ${VERSION} ${name} signature changed; refusing an unsafe IME patch`);
  }
  const first = bundle.indexOf(candidate);
  return `${bundle.slice(0, first)}${patched}${bundle.slice(first + candidate.length)}`;
}

function patchXtermImeBundle(bundle) {
  return replaceExactly(
    replaceExactly(
      replaceExactly(
        replaceExactly(bundle, ORIGINAL_INPUT_EVENT, PATCHED_INPUT_EVENT, "input event", [PREVIOUS_PATCHED_INPUT_EVENT, LEGACY_PATCHED_INPUT_EVENT]),
        ORIGINAL_COMPOSITION,
        PATCHED_COMPOSITION,
        "composition helper",
        [PREVIOUS_PATCHED_COMPOSITION, LEGACY_PATCHED_COMPOSITION],
      ),
      ORIGINAL_FINALIZE_AND_DIFF,
      PATCHED_FINALIZE,
      "composition finalizer",
      [PREVIOUS_PATCHED_FINALIZE],
    ),
    ORIGINAL_COMPOSITION_END_LISTENER,
    PATCHED_COMPOSITION_END_LISTENER,
    "composition-end listener",
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
