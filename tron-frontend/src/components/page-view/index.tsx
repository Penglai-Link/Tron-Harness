import { useEffect, useRef } from "react";

import {
    TargetPageFrame,
    type TargetPageFrameHandle,
    type TargetPageFrameInspectionResult,
} from "./TargetPageFrame";

export interface PageViewProps {
    inspectionMode?: boolean;
    onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
    onUrlChange?: (url: string) => void;
    showChrome?: boolean;
    url: string;
}

export function PageView({ inspectionMode = false, onInspectionConfirm, onUrlChange, showChrome = true, url }: PageViewProps) {
    const frameRef = useRef<TargetPageFrameHandle | null>(null);

    useEffect(() => {
        const normalizedUrl = url.trim();
        frameRef.current?.open(normalizedUrl);
    }, [url]);

    return <TargetPageFrame inspectionMode={inspectionMode} ref={frameRef} onInspectionConfirm={inspectionMode ? onInspectionConfirm : undefined} onUrlChange={onUrlChange} showChrome={showChrome} />;
}

export type { TargetPageFrameInspectionResult };
