import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { LucideIcon } from "lucide-react";

interface IconButtonProps {
  onClick?: () => void;
  size?: number;
  className?: string;
  disabled?: boolean;
  children?: React.ReactNode;
  icon?: LucideIcon;
}

const IconButton = ({
  disabled,
  size = 14,
  className,
  children,
  icon: Icon,
  onClick,
}: IconButtonProps) => {
  return (
    <Button
      disabled={disabled}
      onClick={() => onClick?.()}
      type="button"
      variant="link"
      className={cn("flex gap-1 p-0 hover:no-underline", className)}
    >
      {Icon && <Icon size={size} />}
      <span className={`text-[${size}px]`}>{children}</span>
    </Button>
  );
};

export default IconButton;
