import { isNumber } from "lodash-es"
import { useCallback, useEffect, useRef, useState } from "react"
import type { PanelImperativeHandle, PanelSize } from "react-resizable-panels"

const RIGHT_PANEL_COLLAPSE_THRESHOLD = 300

interface UsePanelOptions {
    collapseThreshold?: number
    initialSize?: number | string
    onOpenChange?: (open: boolean) => void
    onSizeChange?: (size: number) => void
    open: boolean
    resetKey?: string | number | null
}

export function usePanel({
    collapseThreshold = RIGHT_PANEL_COLLAPSE_THRESHOLD,
    initialSize,
    onOpenChange,
    onSizeChange,
    open,
    resetKey = null,
}: UsePanelOptions) {
    const panelRef = useRef<PanelImperativeHandle>(null)
    const onOpenChangeRef = useRef(onOpenChange)
    const onSizeChangeRef = useRef(onSizeChange)
    const panelDraggingRef = useRef(false)
    const panelCollapseRequestedRef = useRef(false)
    const panelLastSizeRef = useRef<number | null>(null)
    const panelNotifiedOpenRef = useRef(open)
    const [panelDragging, setPanelDragging] = useState(false)
    const [dragTimeCollapsed, setDragTimeCollapsed] = useState(false)

    useEffect(() => {
        onOpenChangeRef.current = onOpenChange
    }, [onOpenChange])

    useEffect(() => {
        onSizeChangeRef.current = onSizeChange
    }, [onSizeChange])

    useEffect(() => {
        const frame = window.requestAnimationFrame(() => {
            if (panelDraggingRef.current) {
                return
            }

            if (open) {
                panelRef.current?.expand()
                if (initialSize !== undefined) {
                    panelRef.current?.resize(
                        typeof initialSize === "number" ? `${initialSize}px` : initialSize,
                    )
                } else {
                    const panelElement = document.getElementById("resize-panel")
                    const groupWidth = panelElement?.parentElement?.getBoundingClientRect().width || window.innerWidth
                    const targetPixels = Math.min(680, Math.max(520, groupWidth * 0.42))
                    panelRef.current?.resize(`${targetPixels}px`)
                }
            } else {
                panelRef.current?.collapse()
            }

            panelCollapseRequestedRef.current = false
            panelLastSizeRef.current = null
            panelNotifiedOpenRef.current = open
        })

        return () => window.cancelAnimationFrame(frame)
    }, [initialSize, open, resetKey])

    useEffect(() => {
        if (!panelDraggingRef.current) {
            panelNotifiedOpenRef.current = open
        }
    }, [open])

    const notifyOpenChange = useCallback((nextOpen: boolean) => {
        if (panelNotifiedOpenRef.current === nextOpen) {
            return
        }

        panelNotifiedOpenRef.current = nextOpen
        onOpenChangeRef.current?.(nextOpen)
    }, [])

    const panelResize = useCallback((panelSize: PanelSize) => {
        if (!isNumber(panelSize.inPixels)) {
            return
        }

        panelLastSizeRef.current = panelSize.inPixels
        const shouldCollapse = panelSize.inPixels < collapseThreshold
        panelCollapseRequestedRef.current = shouldCollapse

        if (panelDraggingRef.current && panelSize.inPixels >= collapseThreshold) {
            onSizeChangeRef.current?.(panelSize.inPixels)
        }

        if (panelDraggingRef.current) {
            setDragTimeCollapsed(shouldCollapse)
            notifyOpenChange(!shouldCollapse)
        }
    }, [collapseThreshold, notifyOpenChange])

    const handlePanelPointerDown = useCallback(() => {
        panelDraggingRef.current = true
        setPanelDragging(true)
        panelCollapseRequestedRef.current = false
        panelLastSizeRef.current = panelRef.current?.getSize().inPixels ?? null
        panelNotifiedOpenRef.current = open
    }, [open])

    const handlePanelPointerMove = useCallback(() => {
        if (!panelDraggingRef.current) {
            return
        }

        const currentSize = panelRef.current?.getSize().inPixels ?? panelLastSizeRef.current
        if (!isNumber(currentSize) || currentSize >= collapseThreshold) {
            panelCollapseRequestedRef.current = false
            if (panelRef.current?.isCollapsed()) {
                panelRef.current.expand()
            }
            return
        }

        panelCollapseRequestedRef.current = true
        if (!panelRef.current?.isCollapsed()) {
            panelRef.current?.collapse()
        }
    }, [collapseThreshold])

    const handlePanelPointerEnd = useCallback(() => {
        if (!panelDraggingRef.current && !panelCollapseRequestedRef.current) {
            return
        }

        const currentSize = panelRef.current?.getSize().inPixels
        const size = isNumber(currentSize) ? currentSize : panelLastSizeRef.current
        const shouldCollapse = !isNumber(size) || size < collapseThreshold

        if (shouldCollapse) {
            panelRef.current?.collapse()
            notifyOpenChange(false)
        } else {
            panelRef.current?.expand()
            notifyOpenChange(true)
        }

        panelCollapseRequestedRef.current = false
        panelDraggingRef.current = false
        setPanelDragging(false)
        panelLastSizeRef.current = null

        if (shouldCollapse) {
            return
        }

        if (isNumber(size)) {
            onSizeChangeRef.current?.(size)
        }
    }, [collapseThreshold, notifyOpenChange])

    useEffect(() => {
        window.addEventListener("pointerup", handlePanelPointerEnd)
        window.addEventListener("pointercancel", handlePanelPointerEnd)
        return () => {
            window.removeEventListener("pointerup", handlePanelPointerEnd)
            window.removeEventListener("pointercancel", handlePanelPointerEnd)
        }
    }, [handlePanelPointerEnd])

    return {
        handlePanelPointerDown,
        handlePanelPointerEnd,
        handlePanelPointerMove,
        panelOpen: open || panelDragging,
        panelRef,
        panelResize,
    }
}
