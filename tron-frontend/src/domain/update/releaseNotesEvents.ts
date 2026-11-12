export const OPEN_RELEASE_NOTES_EVENT = "tron:open-release-notes";

export function openReleaseNotes() {
  window.dispatchEvent(new Event(OPEN_RELEASE_NOTES_EVENT));
}
