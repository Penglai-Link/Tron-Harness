import { PromptEditorPage } from "@/domain/prompt/components";

export function PlatformPromptDetailsPage() {
  return <PromptEditorPage mode="detail" scopeKind="PLATFORM" editable={false} />;
}
