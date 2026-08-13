import { cn } from '@/lib/utils'
import React, { useEffect } from 'react'

interface IDraggleLineProps {
    onClick?: React.MouseEventHandler<HTMLButtonElement>,
    className?: string | undefined
    ariaLabel?: string | undefined
}

const DraggleLine = ({ onClick, className, ariaLabel }: IDraggleLineProps) => {

    return (
        <button
            className={cn('group cursor-pointer inline-flex w-px bg-gary-200 items-center justify-center hover:bg-[var(--color-outline-variant)]', className)}
            type="button"
            aria-label={ariaLabel}
            onClick={onClick}
        >
            <div className="flex h-12 w-2 shrink-0 rounded-lg bg-[var(--color-outline-variant)] transition-colors group-hover:bg-[var(--color-outline)] group-aria-[orientation=horizontal]:h-1 group-aria-[orientation=horizontal]:w-6" />
        </button>
    )
}

export default DraggleLine
