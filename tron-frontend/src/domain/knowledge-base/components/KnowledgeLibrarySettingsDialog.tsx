import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Field, TextInput, TextArea } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { RagSettingsFields } from "@/domain/rag/components/RagSettingsFields";
import { canSaveRagSettings } from "@/domain/rag/model/ragSettings";
import { getKnowledgeLibrarySettings, updateKnowledgeLibrarySettings, type KnowledgeLibrarySettings } from "../api/knowledgeBaseApi";
import { useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";
import { DocumentParsingPreferenceField } from "./DocumentParsingPreferenceField";

export function KnowledgeLibrarySettingsDialog({ libraryId, onClose }: { libraryId: string; onClose: () => void }) {
  const { t } = useTranslation("pages");
  const user = useKnowledgeUser();
  const [saving, setSaving] = useState(false);
  const query = useQuery({ queryKey: ["knowledge-library-settings", user, libraryId], queryFn: ({ signal }) => getKnowledgeLibrarySettings(libraryId, signal), staleTime: 0, gcTime: 0, refetchOnMount: "always", refetchOnWindowFocus: false, refetchOnReconnect: false });
  return <Dialog open onOpenChange={(open) => { if (!open && !saving) onClose(); }}>
    <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-2xl" closeLabel={t("knowledgeLibrary.cancel")}>
      <DialogHeader><DialogTitle>{t("knowledgeLibrary.librarySettings")}{query.data ? ` · ${query.data.name}` : ""}</DialogTitle>
        <DialogDescription>{t("knowledgeLibrary.librarySettingsHelp")}</DialogDescription>
      </DialogHeader>
      {query.isPending || query.isFetching ? <p role="status">{t("knowledgeLibrary.loading")}</p> : query.error ? <div className="grid gap-3"><p role="alert">{query.error.message}</p><Button variant="outline" onClick={() => void query.refetch()}>{t("knowledgeLibrary.refresh")}</Button></div> : query.data ?
        <SettingsForm initial={query.data} libraryId={libraryId} onClose={onClose} onSaving={setSaving} /> : null}
    </DialogContent>
  </Dialog>;
}

function SettingsForm({ initial, libraryId, onClose, onSaving }: { initial: KnowledgeLibrarySettings; libraryId: string; onClose: () => void; onSaving: (saving: boolean) => void }) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const [settings, setSettings] = useState(() => structuredClone(initial));
  const cache = useQueryClient();
  const user = useKnowledgeUser();
  const active = useKnowledgeViewActive();
  const toast = useToast();
  const normalized = { ...settings, name: settings.name.trim(), description: settings.description.trim() };
  const changed = JSON.stringify(normalized) !== JSON.stringify(initial);
  const save = useMutation({ mutationFn: () => updateKnowledgeLibrarySettings(libraryId, normalized),
    onMutate: () => onSaving(true),
    onSuccess: async (value) => {
      cache.setQueryData(["knowledge-library-settings", user, libraryId], value);
      await Promise.all([
        cache.invalidateQueries({ queryKey: ["platform-knowledge-bases", user] }),
        cache.invalidateQueries({ queryKey: ["knowledge-base", user, libraryId] }),
        cache.invalidateQueries({ queryKey: ["knowledge-bases", user] }),
        cache.invalidateQueries({ queryKey: ["rag-corpora", user] }),
        cache.invalidateQueries({ queryKey: ["rag-tasks", user] }),
      ]);
      if (active.current) { toast.success(text("librarySettingsSaved")); onClose(); }
    },
    onError: (error) => { if (active.current) toast.error(error.message); },
    onSettled: () => onSaving(false),
  });
  return <form className="grid gap-4" onSubmit={(event) => { event.preventDefault(); if (changed && normalized.name && canSaveRagSettings(settings.indexSettings) && !save.isPending) save.mutate(); }}>
    <Field label={text("name")} required><TextInput aria-label={text("name")} value={settings.name} onChange={(event) => setSettings({ ...settings, name: event.target.value })} maxLength={120} required disabled={save.isPending} /></Field>
    <Field label={text("description")}><TextArea aria-label={text("description")} value={settings.description} onChange={(event) => setSettings({ ...settings, description: event.target.value })} maxLength={2000} disabled={save.isPending} /></Field>
    <DocumentParsingPreferenceField value={settings.defaultParsePreference} onChange={(value) => setSettings({ ...settings, defaultParsePreference: value })} disabled={save.isPending} />
    <p className="text-sm text-[var(--color-on-surface-variant)]">{text("libraryParsingSettingsHelp")}</p>
    <section aria-labelledby="knowledge-edit-index-settings" className="grid gap-4 border-t pt-4">
      <h3 id="knowledge-edit-index-settings" className="font-medium">{text("indexSettings")}</h3>
      <p className="text-sm text-[var(--color-on-surface-variant)]">{text("libraryIndexSettingsHelp")}</p>
      <RagSettingsFields settings={settings.indexSettings} setSettings={(value) => setSettings({ ...settings, indexSettings: value })} knowledgeBase disabled={save.isPending} idPrefix="knowledge-edit-rag" className="sm:grid-cols-2" />
    </section>
    <DialogFooter><Button type="button" variant="outline" disabled={save.isPending} onClick={onClose}>{text("cancel")}</Button><Button type="submit" disabled={!changed || !normalized.name || !canSaveRagSettings(settings.indexSettings) || save.isPending}>{text(save.isPending ? "saving" : "save")}</Button></DialogFooter>
  </form>;
}
