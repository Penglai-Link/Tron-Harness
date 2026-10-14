import { useState } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Field, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { CodeEditor } from "@/components/files/CodeEditor";
import type { SoulState } from "../api/dudeApi";
export function SoulFields({
  name,
  markdown,
  state,
  disabled,
  onName,
  onMarkdown,
  onSave,
}: {
  name: string;
  markdown: string;
  state: SoulState;
  disabled?: boolean;
  onName: (v: string) => void;
  onMarkdown: (v: string) => void;
  onSave: () => void;
}) {
  const { t } = useTranslation("pages");
  const [preview, setPreview] = useState(false);
  return (
    <div className="space-y-4">
      <p
        role="status"
        className="text-sm text-[var(--color-on-surface-variant)]"
      >
        {t(`dude.soulStates.${state}`)}
      </p>
      <Field label={t("dude.name")} required>
        <TextInput
          aria-label={t("dude.name")}
          value={name}
          disabled={disabled}
          onChange={(e) => onName(e.target.value)}
        />
      </Field>
      <div className="flex items-center justify-between">
        <span>{t("dude.soulBody")}</span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => setPreview((v) => !v)}
        >
          {t(preview ? "dude.soulEdit" : "dude.soulPreview")}
        </Button>
      </div>
      <div className="h-64 overflow-auto rounded-md border border-[var(--color-outline-variant)]">
        {preview ? (
          <div className="prose prose-sm p-4 text-[var(--color-on-surface)]">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {markdown}
            </ReactMarkdown>
          </div>
        ) : (
          <CodeEditor
            path="dude-soul.md"
            value={markdown}
            readOnly={disabled}
            onChange={onMarkdown}
            onSave={onSave}
          />
        )}
      </div>
    </div>
  );
}
