import {
  FileArchive,
  RefreshCcw,
  Settings2,
} from "lucide-react";

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
  return (
    <div className="grid gap-5 xl:grid-cols-3">
      <SurfaceCard>
        <SectionTitle
          icon={<Settings2 size={17} aria-hidden="true" />}
          title="Debug Profile"
        />
        <div className="mt-4 grid gap-3">
          <Field label="Profile ID">
            <SelectInput defaultValue="project-doc-related-v1">
              <option value="project-doc-related-v1">project-doc-related-v1</option>
              <option value="memory-related-v1">memory-related-v1</option>
              <option value="skill-related-v1">skill-related-v1</option>
            </SelectInput>
          </Field>
          <Button variant="secondary" onClick={() => onNotice("Profile debug response preview refreshed.")}>
            Load Profile
          </Button>
        </div>
        <DiagnosticPreview rows={[["Routes", "FULLTEXT, VECTOR"], ["Strategy", "DOC_QA_CONTEXT_PACK"], ["Warnings", "2"]]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<FileArchive size={17} aria-hidden="true" />}
          title="Debug Source"
        />
        <div className="mt-4 grid gap-3">
          <Field label="Source Code">
            <TextInput defaultValue="docs/features/rag-search-context-pack" />
          </Field>
          <Button variant="secondary" onClick={() => onNotice("Source debug response preview refreshed.")}>
            Load Source
          </Button>
        </div>
        <DiagnosticPreview rows={[["Status", "Active"], ["Text Units", "348"], ["Last Indexed", "8 min ago"]]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<RefreshCcw size={17} aria-hidden="true" />}
          title="Retry Index"
        />
        <div className="mt-4 grid gap-3">
          <Field label="Stage">
            <SelectInput defaultValue="vectorIndex">
              <option value="fulltextIndex">fulltextIndex</option>
              <option value="vectorIndex">vectorIndex</option>
            </SelectInput>
          </Field>
          <Field label="Reason">
            <TextArea className="min-h-24" defaultValue="Backfill missing embeddings after profile validation." />
          </Field>
          <Button onClick={() => onNotice("Index retry prototype request queued.")}>
            Queue Retry
          </Button>
        </div>
      </SurfaceCard>
    </div>
  );
}
