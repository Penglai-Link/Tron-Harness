import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { SearchField } from "@/components/search-field";
import { useToast } from "@/components/ui/toast";
import { fetchSkillAgents, setSkillAgents } from "@/domain/skill/api";
import type { RegistrySkillRecord } from "@/domain/skill/model";
import { listAssistants } from "@/domain/assistant/api/assistantApi";
import { fetchWorkspaceInternalAgents } from "@/domain/agent/api/agentManagementApi";
import { PLATFORM_INTERNAL_AGENT_SCOPE } from "@/domain/agent/model";

export function SkillAgentGrants({
  skill,
  onClose,
  onSaved,
}: {
  skill: RegistrySkillRecord;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const client = useQueryClient();
  const [agents, setAgents] = useState<{ agentCode: string; name: string }[]>(
    []
  );
  const [selected, setSelected] = useState<string[]>([]);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const text = (k: string) => t(`agentSkills.${k}`);
  const generation = useRef(0);
  async function load() {
    const current = ++generation.current;
    setLoading(true);
    try {
      const [a, g, assistants] = await Promise.all([
        fetchWorkspaceInternalAgents(PLATFORM_INTERNAL_AGENT_SCOPE),
        fetchSkillAgents(skill.skillName),
        listAssistants(),
      ]);
      if (current !== generation.current) return;
      setAgents([...a, ...assistants.map(assistant => ({ agentCode: assistant.assistantCode, name: assistant.name }))]);
      setSelected(g.agentCodes);
      setError("");
    } catch (e) {
      if (current === generation.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (current === generation.current) setLoading(false);
    }
  }
  useEffect(() => {
    void load();
    return () => { generation.current += 1; };
  }, [skill.skillName]);
  async function save() {
    setBusy(true);
    try {
      await setSkillAgents(skill.skillName, selected);
      await client.invalidateQueries({ queryKey: ["agent-skills"] });
      onSaved();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog
      open
      disablePointerDismissal
      onOpenChange={(o) => {
        if (!o && !busy) onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{text("assignAgents")}</DialogTitle>
          <DialogDescription>{skill.skillName}</DialogDescription>
        </DialogHeader>
        <SearchField
          ariaLabel={text("searchAgents")}
          placeholder={text("searchAgents")}
          value={search}
          onChange={setSearch}
        />
        {error ? (
          <p role="alert">
            {error}
            <Button onClick={() => void load()}>{text("retry")}</Button>
          </p>
        ) : null}
        <div className="max-h-[50dvh] space-y-2 overflow-y-auto">
          {agents
            .filter((a) =>
              `${a.name} ${a.agentCode}`
                .toLowerCase()
                .includes(search.toLowerCase())
            )
            .map((a) => (
              <label
                key={a.agentCode}
                className="flex items-center gap-3 rounded-xl border border-[var(--color-outline-variant)] p-3"
              >
                <input
                  type="checkbox"
                  disabled={busy}
                  checked={selected.includes(a.agentCode)}
                  onChange={(e) =>
                    setSelected(
                      e.currentTarget.checked
                        ? [...selected, a.agentCode]
                        : selected.filter((c) => c !== a.agentCode)
                    )
                  }
                />
                <span>
                  {a.name}
                  <span className="block text-xs">{a.agentCode}</span>
                </span>
              </label>
            ))}
        </div>
        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            {text("cancel")}
          </Button>
          <Button
            disabled={busy || loading || !!error}
            onClick={() => void save()}
          >
            {text("save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
