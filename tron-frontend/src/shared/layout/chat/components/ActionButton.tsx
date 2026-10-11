import { cn } from "@/lib/utils";

interface ActionButtonProps {
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  className?: string;
  disabled?: boolean;
  children?: React.ReactNode;
  needHover?: boolean;
  "aria-label"?: string;
}

const ActionButton = ({
  disabled,
  className,
  children,
  needHover,
  onClick,
  ...rest
}: ActionButtonProps) => {
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className={cn(`flex gap-1 items-center p-0 ${needHover ? 'opacity-60 hover:opacity-100' : ''}`, className)}
      {...rest}
    >
      {children ?? rest["aria-label"]}
    </button>
  );
};

export default ActionButton;
