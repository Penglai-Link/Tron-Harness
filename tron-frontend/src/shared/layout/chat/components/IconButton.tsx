import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { LucideIcon } from "lucide-react";

interface IconButtonProps {
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  size?: "xs" | "sm" | "lg";
  className?: string;
  disabled?: boolean;
  children?: React.ReactNode;
  icon?: LucideIcon;
}

const IconButton = ({
  disabled,
  size = "sm",
  className,
  children,
  icon: Icon,
  onClick,
}: IconButtonProps) => {
  return (
    <Button
      disabled={disabled}
      onClick={onClick}
      size={size}
      type="button"
      variant="link"
      className={cn("flex gap-1 p-0 hover:no-underline", className)}
    >
      {Icon && <Icon />}
      {children}
    </Button>
  );
};

export default IconButton;
