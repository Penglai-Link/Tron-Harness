import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

export function ChatAttachmentButton({ disabled, uploading, onClick }: {
  disabled?: boolean;
  uploading?: boolean;
  onClick: () => void;
}) {
  const { t } = useTranslation("pages");
  return <Button
    aria-label={t("chat.management.composer.uploadAttachment")}
    disabled={disabled || uploading}
    size="icon-sm"
    title={t(uploading ? "chat.management.composer.uploading" : "chat.management.composer.uploadAttachment")}
    type="button"
    variant="ghost"
    onClick={onClick}
  >
    <Plus aria-hidden="true" className="!block size-4" />
  </Button>;
}
