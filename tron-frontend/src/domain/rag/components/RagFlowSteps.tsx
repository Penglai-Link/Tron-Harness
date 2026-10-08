import { ArrowDown, ArrowRight, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Card, CardContent } from "@/components/ui/card";

export function RagFlowSteps({ label, steps }: {
  label: string;
  steps: readonly { stage: string; icon: LucideIcon }[];
}) {
  const { t } = useTranslation("pages");
  return <ol aria-label={label} className="grid gap-8 lg:grid-cols-4">
    {steps.map(({ stage, icon: Icon }, index) => <li key={stage} className="relative min-w-0">
      <Card className="h-full gap-0 rounded-lg bg-muted/40 py-4 shadow-none">
        <CardContent className="px-4">
          <div className="flex items-center gap-2"><Icon size={18} className="shrink-0 text-muted-foreground" aria-hidden="true" /><h3 className="font-medium">{t(`ragUnified.flow.${stage}`)}</h3></div>
          <p className="mt-2 text-sm text-muted-foreground">{t(`ragUnified.flow.${stage}Help`)}</p>
        </CardContent>
      </Card>
      {index < steps.length - 1 ? <span aria-hidden="true" className="pointer-events-none absolute -bottom-6 left-1/2 -translate-x-1/2 text-muted-foreground lg:-right-6 lg:bottom-auto lg:left-auto lg:top-1/2 lg:-translate-y-1/2 lg:translate-x-0">
        <ArrowDown size={16} className="lg:hidden" /><ArrowRight size={16} className="hidden lg:block" />
      </span> : null}
    </li>)}
  </ol>;
}
