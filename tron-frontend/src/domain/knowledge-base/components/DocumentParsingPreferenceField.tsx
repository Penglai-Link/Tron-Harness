import { useTranslation } from "react-i18next";
import { Field, SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import type { DocumentParsePreference } from "../model/KnowledgeLibraryModel";

export function DocumentParsingPreferenceField({ value, onChange, disabled }: { value: DocumentParsePreference; onChange: (value: DocumentParsePreference) => void; disabled?: boolean }) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  return <Field label={text("parsingPreference")} hintAriaLabel={text("parsingPreference")} hint={text("autoParsingHelp")}>
    <SelectInput aria-label={text("parsingPreference")} value={value} disabled={disabled} onValueChange={(value) => onChange(value as DocumentParsePreference)}>
      {(["Auto", "Native", "Recognition", "Layout"] as const).map((mode) => <SelectItem value={mode} key={mode}>{text(`parsingPreferences.${mode}`)}</SelectItem>)}
    </SelectInput>
  </Field>;
}
