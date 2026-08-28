import { useEffect, useRef } from "react";

import {
    TargetPageFrame,
    type TargetPageFrameHandle,
    type TargetPageFrameInspectionResult,
} from "./TargetPageFrame";

export interface PageViewProps {
    onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
    showChrome?: boolean;
    url: string;
}

export function PageView({ onInspectionConfirm, showChrome = true, url }: PageViewProps) {
    const frameRef = useRef<TargetPageFrameHandle | null>(null);

    useEffect(() => {
        const normalizedUrl = url.trim();
        if (!normalizedUrl) {
            return;
        }
        frameRef.current?.open(normalizedUrl);
    }, [url]);

    return <TargetPageFrame ref={frameRef} onInspectionConfirm={onInspectionConfirm} showChrome={showChrome} />;
}

export type { TargetPageFrameInspectionResult };
export { LinkBrowserDialog } from "./LinkBrowserDialog";
