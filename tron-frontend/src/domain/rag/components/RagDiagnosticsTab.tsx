import {
  FileArchive,
  RefreshCcw,
  Settings2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Field,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  DiagnosticPreview,
  SectionTitle,
} from "@/domain/rag/components/RagShared";

export function RagDiagnosticsTab({ onNotice }: { onNotice: (notice: string) => void }) {
  const { t } = useTranslation("pages");
  const pageT = (key: string, defaultValue: string) => t(`rag.diagnostics.${key}`, { defaultValue });

  return (
    <div className="grid gap-5 xl:grid-cols-3">
      <SurfaceCard>
        <SectionTitle
          icon={<Settings2 size={17} aria-hidden="true" />}
          title={pageT("config.title", "Debug profile")}
        />
        <div className="mt-4 grid gap-3">
          <Field label={pageT("config.profileId", "Profile ID")}>
            <SelectInput defaultValue="project-doc-related-v1">
              <option value="project-doc-related-v1">{pageT("config.options.projectDoc", "Project document relevance profile")}</option>
              <option value="memory-related-v1">{pageT("config.options.memory", "Memory relevance profile")}</option>
              <option value="skill-related-v1">{pageT("config.options.skill", "Skill relevance profile")}</option>
            </SelectInput>
          </Field>
          <Button variant="secondary" onClick={() => onNotice(pageT("messages.configLoaded", "Profile debug preview refreshed."))}>
            {pageT("config.load", "Load profile")}
          </Button>
        </div>
        <DiagnosticPreview rows={[
          [pageT("preview.route", "Route"), pageT("preview.fulltextVector", "Fulltext, vector")],
          [pageT("preview.strategy", "Strategy"), pageT("preview.docQaPack", "Document QA context pack")],
          [pageT("preview.warnings", "Warnings"), "2"],
        ]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<FileArchive size={17} aria-hidden="true" />}
          title={pageT("source.title", "Debug source")}
        />
        <div className="mt-4 grid gap-3">
          <Field label={pageT("source.sourceCode", "Source code")}>
            <TextInput defaultValue="docs/features/rag-search-context-pack" />
          </Field>
          <Button variant="secondary" onClick={() => onNotice(pageT("messages.sourceLoaded", "Source debug preview refreshed."))}>
            {pageT("source.load", "Load source")}
          </Button>
        </div>
        <DiagnosticPreview rows={[
          [pageT("preview.status", "Status"), pageT("preview.active", "Active")],
          [pageT("preview.textUnits", "Text units"), "348"],
          [pageT("preview.lastIndexed", "Last indexed"), pageT("preview.eightMinutesAgo", "8 minutes ago")],
        ]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<RefreshCcw size={17} aria-hidden="true" />}
          title={pageT("retry.title", "Retry indexing")}
        />
        <div className="mt-4 grid gap-3">
          <Field label={pageT("retry.stage", "Stage")}>
            <SelectInput defaultValue="vectorIndex">
              <option value="fulltextIndex">{pageT("retry.options.fulltextIndex", "Fulltext index")}</option>
              <option value="vectorIndex">{pageT("retry.options.vectorIndex", "Vector index")}</option>
            </SelectInput>
          </Field>
          <Field label={pageT("retry.reason", "Reason")}>
            <TextArea className="min-h-24" defaultValue={pageT("retry.defaultReason", "Backfill missing vector indexes after profile validation passes.")} />
          </Field>
          <Button onClick={() => onNotice(pageT("messages.retryQueued", "Index retry request queued."))}>
            {pageT("retry.queue", "Queue retry")}
          </Button>
        </div>
      </SurfaceCard>
    </div>
  );
}
