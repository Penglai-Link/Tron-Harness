import { useCallback, useEffect, useRef } from "react";

import {
    TargetPageFrame,
    type TargetPageFrameHandle,
    type TargetPageFrameInspectionResult,
} from "./TargetPageFrame";

export interface PageViewProps {
  inspectionMode?: boolean;
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
  onUrlChange?: (url: string) => void;
  pageId?: string;
  partition?: string;
  url: string;
}

export function PageView({ inspectionMode = false, onInspectionConfirm, onUrlChange, pageId, partition, url }: PageViewProps) {
    const frameRef = useRef<TargetPageFrameHandle | null>(null);
    const reportedUrlRef = useRef("");

    useEffect(() => {
        const normalizedUrl = url.trim();
        if (reportedUrlRef.current === normalizedUrl) {
            reportedUrlRef.current = "";
            return;
        }
        frameRef.current?.open(normalizedUrl);
    }, [url]);

    const handleUrlChange = useCallback((nextUrl: string) => {
        reportedUrlRef.current = nextUrl.trim();
        onUrlChange?.(nextUrl);
    }, [onUrlChange]);

    return <TargetPageFrame initialUrl={url} inspectionMode={inspectionMode} ref={frameRef} onInspectionConfirm={inspectionMode ? onInspectionConfirm : undefined} onUrlChange={handleUrlChange} pageId={pageId} partition={partition} />;
}

export type { TargetPageFrameInspectionResult };
