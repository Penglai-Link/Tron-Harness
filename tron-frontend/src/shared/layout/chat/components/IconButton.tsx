import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { LucideIcon } from "lucide-react";

interface IconButtonProps {
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  size?: number;
  className?: string;
  disabled?: boolean;
  children?: React.ReactNode;
  icon?: LucideIcon;
  needHover?: boolean;
}

const IconButton = ({
  disabled,
  size = 16,
  className,
  children,
  icon: Icon,
  needHover,
  onClick,
  ...rest
}: IconButtonProps) => {
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className={cn(`flex gap-1 items-center p-0 ${needHover ? 'opacity-60 hover:opacity-100' : ''}`, className)}
      style={{ fontSize: size }}
      {...rest}
    >
      {Icon && <Icon size={size} />}
      {children}
    </button>
  );
};

export default IconButton;
