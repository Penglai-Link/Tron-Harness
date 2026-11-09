import { isNumber } from "lodash-es"
import { useCallback, useEffect, useRef, useState } from "react"
import type { PanelImperativeHandle, PanelSize } from "react-resizable-panels"

const RIGHT_PANEL_COLLAPSE_THRESHOLD = 300

interface UsePanelOptions {
    adaptToWidth?: boolean
    collapseThreshold?: number
    initialSize?: number | string
    onOpenChange?: (open: boolean) => void
    onSizeChange?: (size: number) => void
    open: boolean
    resetKey?: string | number | null
}

export function usePanel({
    adaptToWidth = false,
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
                    panelRef.current?.resize("42%")
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

    // The same drag gesture must work when the main workspace is narrow.
    const effectiveCollapseThreshold = useCallback(() => {
        if (!adaptToWidth) return collapseThreshold
        const groupWidth = document.getElementById("resize-panel")?.parentElement?.getBoundingClientRect().width
        return groupWidth ? Math.min(collapseThreshold, groupWidth * 0.2) : collapseThreshold
    }, [adaptToWidth, collapseThreshold])

    const panelResize = useCallback((panelSize: PanelSize) => {
        if (!isNumber(panelSize.inPixels)) {
            return
        }

        panelLastSizeRef.current = panelSize.inPixels
        const shouldCollapse = panelSize.inPixels < effectiveCollapseThreshold()
        panelCollapseRequestedRef.current = shouldCollapse

        if (panelDraggingRef.current && panelSize.inPixels >= effectiveCollapseThreshold()) {
            onSizeChangeRef.current?.(panelSize.inPixels)
        }

        if (panelDraggingRef.current) {
            setDragTimeCollapsed(shouldCollapse)
            notifyOpenChange(!shouldCollapse)
        }
    }, [effectiveCollapseThreshold, notifyOpenChange])

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
        if (!isNumber(currentSize) || currentSize >= effectiveCollapseThreshold()) {
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
    }, [effectiveCollapseThreshold])

    const handlePanelPointerEnd = useCallback(() => {
        if (!panelDraggingRef.current && !panelCollapseRequestedRef.current) {
            return
        }

        const currentSize = panelRef.current?.getSize().inPixels
        const size = isNumber(currentSize) ? currentSize : panelLastSizeRef.current
        const shouldCollapse = !isNumber(size) || size < effectiveCollapseThreshold()

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
    }, [effectiveCollapseThreshold, notifyOpenChange])

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
