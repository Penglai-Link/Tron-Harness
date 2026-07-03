import { useCallback, useEffect, useRef, useState } from "react"
import { PanelImperativeHandle, PanelSize } from "react-resizable-panels"
import { useLocation } from 'react-router';

const RIGHT_PANEL_COLLAPSE_THRESHOLD = 300

export function usePanel() {
    // 侧边栏展开或者折叠
    const [rightDragglePanelOpen, setRightDragglePanelOpen] = useState(false);
    const panelRef = useRef<PanelImperativeHandle>(null)
    const rightPanelDraggingRef = useRef(false)
    const rightPanelCollapseRequestedRef = useRef(false)
    const rightPanelHandleRef = useRef<HTMLDivElement | null>(null)

    const location = useLocation();

    const expandPanel = useCallback(() => {
        const panelElement = document.getElementById("resize-panel");
        const groupWidth = panelElement?.parentElement?.getBoundingClientRect().width || window.innerWidth;
        const targetPixels = Math.min(680, Math.max(520, groupWidth * 0.42));

        setRightDragglePanelOpen(true)
        panelRef.current?.expand()
        panelRef.current?.resize(`${targetPixels}px`)
    }, []);

    useEffect(() => {
        panelRef.current?.collapse()
        setRightDragglePanelOpen(false)
        rightPanelDraggingRef.current = false
        rightPanelCollapseRequestedRef.current = false
        rightPanelHandleRef.current = null
    }, [location]);

    useEffect(() => {
        const handleOpenRightSidebar = () => {
            window.requestAnimationFrame(expandPanel)
        };

        window.addEventListener("tron:open-right-sidebar", handleOpenRightSidebar);
        return () => window.removeEventListener("tron:open-right-sidebar", handleOpenRightSidebar);
    }, [expandPanel]);

    const panelResize = useCallback((panelSize: PanelSize) => {
        if (panelRef.current?.isCollapsed()) {
            setRightDragglePanelOpen(false)
        }
    }, []);

    const handleRightPanelPointerDown = useCallback(() => {
        rightPanelDraggingRef.current = true
        rightPanelCollapseRequestedRef.current = false
    }, []);

    const bindRightPanelHandleRef = useCallback((element: HTMLDivElement | null) => {
        rightPanelHandleRef.current = element
    }, []);

    const handleRightPanelPointerMove = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
        if (!rightPanelDraggingRef.current || rightPanelCollapseRequestedRef.current) {
            return
        }

        const currentSize = panelRef.current?.getSize().inPixels ?? 0
        if (currentSize > RIGHT_PANEL_COLLAPSE_THRESHOLD) {
            return
        }

        rightPanelCollapseRequestedRef.current = true
        event.preventDefault()
        event.stopPropagation()

        const target = (rightPanelHandleRef.current ?? event.currentTarget) as HTMLDivElement & {
            releasePointerCapture?: (pointerId: number) => void
            hasPointerCapture?: (pointerId: number) => boolean
        }

        if (target.hasPointerCapture?.(event.pointerId)) {
            target.releasePointerCapture?.(event.pointerId)
        }

        panelRef.current?.collapse()
        setRightDragglePanelOpen(false)
        rightPanelDraggingRef.current = false

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
    }, []);

    const handleRightPanelPointerEnd = useCallback(() => {
        rightPanelDraggingRef.current = false
        rightPanelCollapseRequestedRef.current = false
        if (panelRef.current?.isCollapsed()) {
            setRightDragglePanelOpen(false)
        }
    }, []);


    function changeDragglePane() {
        const nextOpen = !rightDragglePanelOpen
        setRightDragglePanelOpen(nextOpen)

        if (nextOpen) {
            panelRef.current?.resize(400)
            panelRef.current?.expand()
        } else {
            panelRef.current?.collapse()
        }
    }

    return {
        expandPanel,
        panelRef,
        rightDragglePanelOpen,
        setRightDragglePanelOpen,
        panelResize,
        changeDragglePane,
        handleRightPanelPointerDown,
        bindRightPanelHandleRef,
        handleRightPanelPointerMove,
        handleRightPanelPointerEnd,
    }
}
