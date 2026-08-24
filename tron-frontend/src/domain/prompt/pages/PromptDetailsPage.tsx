import { PromptEditorPage } from "@/domain/prompt/components";

export function PromptDetailsPage() {
  return <PromptEditorPage mode="detail" scopeKind="WORKSPACE" editable={false} />;
}
