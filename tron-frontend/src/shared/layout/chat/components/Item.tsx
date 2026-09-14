import React from "react";

interface ItemProps {
  left?: React.ReactNode;
  right?: React.ReactNode;
  className?: string;
  onClick?: () => void;
}

const Item = ({ left, right, className, onClick }: ItemProps) => {
  return (
    <div
      className={`px-2 h-8 flex text-sm rounded hover:bg-sidebar-accent hover:text-sidebar-accent-foreground justify-between items-center ${className || ""}`}
      onClick={onClick}
    >
      {left ? left : null}
      {right ? right : null}
    </div>
  );
};

export default Item;
