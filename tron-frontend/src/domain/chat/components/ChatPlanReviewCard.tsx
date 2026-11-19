import { ClipboardList } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";

interface ChatPlanReviewCardProps {
  approving: boolean;
  approveDisabled?: boolean;
  onApprove: () => void;
  onKeepPlanning: () => void;
}

/** Review offered after a plan-mode turn produced a plan; approval leaves plan mode. */
export function ChatPlanReviewCard({ approving, approveDisabled, onApprove, onKeepPlanning }: ChatPlanReviewCardProps) {
  const { t } = useTranslation("pages");
  return (
    <Card
      size="sm"
      role="region"
      aria-label={t("chat.management.planReview.title")}
      className="mx-1 mb-2 gap-2"
    >
      <CardHeader className="gap-1">
        <CardTitle className="flex items-center gap-2">
          <ClipboardList aria-hidden="true" className="size-4 shrink-0 text-[var(--color-accent)]" />
          <span>{t("chat.management.planReview.title")}</span>
        </CardTitle>
        <CardDescription>{t("chat.management.planReview.description")}</CardDescription>
      </CardHeader>
      <CardFooter className="justify-end gap-2 border-0 bg-transparent px-(--card-spacing) py-0">
        <Button type="button" variant="ghost" size="sm" onClick={onKeepPlanning} disabled={approving}>
          {t("chat.management.planReview.keepPlanning")}
        </Button>
        <Button type="button" size="sm" onClick={onApprove} disabled={approving || approveDisabled}>
          {approving ? t("chat.management.planReview.approving") : t("chat.management.planReview.approve")}
        </Button>
      </CardFooter>
    </Card>
  );
}
