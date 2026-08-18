import { useCallback, useEffect, useRef, useState } from "react"
import type { PanelImperativeHandle, PanelSize } from "react-resizable-panels"

const RIGHT_PANEL_COLLAPSE_THRESHOLD = 300
const RIGHT_PANEL_OPEN_EVENT = "tron:open-right-sidebar"
const RIGHT_PANEL_TOGGLE_EVENT = "tron:toggle-right-sidebar"

interface UsePanelOptions {
    collapseThreshold?: number
    initiallyOpen?: boolean
    onOpenChange?: (open: boolean) => void
    openEventName?: string | null
    resetKey?: string | number | null
    toggleEventName?: string | null
}

export function usePanel({
    collapseThreshold = RIGHT_PANEL_COLLAPSE_THRESHOLD,
    initiallyOpen = false,
    onOpenChange,
    openEventName = RIGHT_PANEL_OPEN_EVENT,
    resetKey = null,
    toggleEventName = RIGHT_PANEL_TOGGLE_EVENT,
}: UsePanelOptions = {}) {
    const [panelOpen, setPanelOpen] = useState(initiallyOpen)
    const panelRef = useRef<PanelImperativeHandle>(null)
    const panelDraggingRef = useRef(false)
    const panelCollapseRequestedRef = useRef(false)
    const panelHandleRef = useRef<HTMLDivElement | null>(null)

    const expandPanel = useCallback(() => {
        const panelElement = document.getElementById("resize-panel")
        const groupWidth = panelElement?.parentElement?.getBoundingClientRect().width || window.innerWidth
        const targetPixels = Math.min(680, Math.max(520, groupWidth * 0.42))

        setPanelOpen(true)
        panelRef.current?.expand()
        panelRef.current?.resize(`${targetPixels}px`)
    }, [])

    useEffect(() => {
        const frame = window.requestAnimationFrame(() => {
            // Initial sizes come from ResizablePanel; imperative APIs are only
            // needed for explicit resets after the group has registered panels.
            if (resetKey !== null) {
                if (initiallyOpen) {
                    panelRef.current?.expand()
                } else {
                    panelRef.current?.collapse()
                }
            }

            setPanelOpen(initiallyOpen)
            panelDraggingRef.current = false
            panelCollapseRequestedRef.current = false
            panelHandleRef.current = null
        })

        return () => window.cancelAnimationFrame(frame)
    }, [initiallyOpen, resetKey])

    useEffect(() => {
        if (!openEventName) {
            return
        }

        const handleOpenPanel = () => {
            window.requestAnimationFrame(expandPanel)
        }

        window.addEventListener(openEventName, handleOpenPanel)
        return () => window.removeEventListener(openEventName, handleOpenPanel)
    }, [expandPanel, openEventName])

    const panelResize = useCallback((panelSize: PanelSize) => {
        if (panelRef.current?.isCollapsed()) {
            setPanelOpen(false)
            return
        }

        if (panelSize.inPixels < collapseThreshold) {
            panelCollapseRequestedRef.current = true
        }
    }, [collapseThreshold])

    const handleRightPanelPointerDown = useCallback(() => {
        panelDraggingRef.current = true
        panelCollapseRequestedRef.current = false
    }, [])

    const bindRightPanelHandleRef = useCallback((element: HTMLDivElement | null) => {
        panelHandleRef.current = element
    }, [])

    const handleRightPanelPointerMove = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
        if (!panelDraggingRef.current || panelCollapseRequestedRef.current) {
            return
        }

        const currentSize = panelRef.current?.getSize().inPixels ?? 0
        if (currentSize > collapseThreshold) {
            return
        }

        panelCollapseRequestedRef.current = true
        event.preventDefault()
        event.stopPropagation()

        const target = (panelHandleRef.current ?? event.currentTarget) as HTMLDivElement & {
            releasePointerCapture?: (pointerId: number) => void
            hasPointerCapture?: (pointerId: number) => boolean
        }

        if (target.hasPointerCapture?.(event.pointerId)) {
            target.releasePointerCapture?.(event.pointerId)
        }

        panelRef.current?.collapse()
        setPanelOpen(false)
        panelDraggingRef.current = false

        const pointerUpEvent = new PointerEvent("pointerup", {
            bubbles: true,
            cancelable: true,
            composed: true,
            button: 0,
            buttons: 0,
            clientX: event.clientX,
            clientY: event.clientY,
            pointerId: event.pointerId,
            pointerType: event.pointerType,
            pressure: 0,
        })
        target.dispatchEvent(pointerUpEvent)
    }, [collapseThreshold])

    const flushPanelCollapse = useCallback(() => {
        if (panelCollapseRequestedRef.current && !panelRef.current?.isCollapsed()) {
            panelRef.current?.collapse()
            setPanelOpen(false)
        }

        panelCollapseRequestedRef.current = false
    }, [])

    useEffect(() => {
        window.addEventListener("pointerup", flushPanelCollapse)
        window.addEventListener("pointercancel", flushPanelCollapse)
        return () => {
            window.removeEventListener("pointerup", flushPanelCollapse)
            window.removeEventListener("pointercancel", flushPanelCollapse)
        }
    }, [flushPanelCollapse])

    const handleRightPanelPointerEnd = useCallback(() => {
        flushPanelCollapse()
        panelDraggingRef.current = false
        if (panelRef.current?.isCollapsed()) {
            setPanelOpen(false)
        }
    }, [flushPanelCollapse])

    const changePanel = useCallback(() => {
        setPanelOpen((currentOpen) => {
            const nextOpen = !currentOpen

            if (nextOpen) {
                panelRef.current?.resize(400)
                panelRef.current?.expand()
            } else {
                panelRef.current?.collapse()
            }

            return nextOpen
        })
    }, [])

    useEffect(() => {
        if (!toggleEventName) {
            return
        }

        window.addEventListener(toggleEventName, changePanel)
        return () => window.removeEventListener(toggleEventName, changePanel)
    }, [changePanel, toggleEventName])

    useEffect(() => {
        onOpenChange?.(panelOpen)
    }, [onOpenChange, panelOpen])

    return {
        bindRightPanelHandleRef,
        changePanel,
        changeDragglePane: changePanel,
        expandPanel,
        handleRightPanelPointerDown,
        handleRightPanelPointerEnd,
        handleRightPanelPointerMove,
        panelOpen,
        panelRef,
        panelResize,
        rightDragglePanelOpen: panelOpen,
        setPanelOpen,
        setRightDragglePanelOpen: setPanelOpen,
    }
}
