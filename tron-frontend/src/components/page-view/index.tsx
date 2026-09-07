import { useEffect, useRef } from "react";

import {
    TargetPageFrame,
    type TargetPageFrameHandle,
    type TargetPageFrameInspectionResult,
} from "./TargetPageFrame";

export interface PageViewProps {
    onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
    onUrlChange?: (url: string) => void;
    showChrome?: boolean;
    url: string;
}

export function PageView({ onInspectionConfirm, onUrlChange, showChrome = true, url }: PageViewProps) {
    const frameRef = useRef<TargetPageFrameHandle | null>(null);

    useEffect(() => {
        const normalizedUrl = url.trim();
        frameRef.current?.open(normalizedUrl);
    }, [url]);

    return <TargetPageFrame ref={frameRef} onInspectionConfirm={onInspectionConfirm} onUrlChange={onUrlChange} showChrome={showChrome} />;
}

export type { TargetPageFrameInspectionResult };
