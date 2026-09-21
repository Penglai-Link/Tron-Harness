import { useEffect, useState } from "react";

import { downloadResourceBinary } from "@/domain/resource/api";
import { buildApiUrl } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { parseAvatarResourceReference } from "@/shared/lib/avatarResource";

const SYSTEM_AVATAR_REFERENCE = "system://avatar";

export function useManagedAvatar(avatarUrl: string | null | undefined) {
  const [src, setSrc] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let disposed = false;
    let objectUrl: string | null = null;
    const resolvedAvatarUrl = avatarUrl?.startsWith("/") ? buildApiUrl(avatarUrl) : avatarUrl;
    const reference = parseAvatarResourceReference(resolvedAvatarUrl);
    const isSystemAvatar = resolvedAvatarUrl === SYSTEM_AVATAR_REFERENCE;

    setSrc(null);
    setLoading(Boolean(reference || isSystemAvatar));

    if (!resolvedAvatarUrl) {
      return () => undefined;
    }

    if (!reference && !isSystemAvatar) {
      setSrc(resolvedAvatarUrl);
      setLoading(false);
      return () => undefined;
    }

    if (isSystemAvatar) {
      setLoading(false);
      return () => undefined;
    }

    if (!isSystemAvatar && getServiceDataSource("resource") !== "api") {
      setLoading(false);
      return () => undefined;
    }

    const controller = new AbortController();
    const download = downloadResourceBinary(reference!, controller.signal);
    void download
      .then(({ blob }) => {
        if (disposed) return;
        objectUrl = URL.createObjectURL(blob);
        setSrc(objectUrl);
      })
      .catch(() => {
        if (!disposed) setSrc(null);
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });

    return () => {
      disposed = true;
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [avatarUrl]);

  return { loading, src };
}
