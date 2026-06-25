import { useCallback, useEffect, useRef } from "react"
import { PanelImperativeHandle, PanelSize } from "react-resizable-panels"
import { useLocation } from 'react-router';

export function usePanel() {
    const panelRef = useRef<PanelImperativeHandle>(null)

    const location = useLocation();

    useEffect(() => {
        panelRef.current?.collapse()
    }, [location]);

    const panelResize = useCallback((panelSize: PanelSize) => {
        if (panelSize.inPixels < 500 && !panelRef.current?.isCollapsed()) {
            panelRef.current?.collapse()
        }
    }, []);

    return {
        panelRef,
        panelResize,
    }
}