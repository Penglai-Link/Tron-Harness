import { useEffect, useState } from "react";

import { downloadAccountAvatar } from "@/domain/account/api";
import { getAuthDataSource } from "@/domain/auth/api/authDataSource";
import { downloadResourceBinary } from "@/domain/resource/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { parseAvatarResourceReference } from "@/shared/lib/avatarResource";

const SYSTEM_AVATAR_REFERENCE = "system://avatar";

export function useManagedAvatar(avatarUrl: string | null | undefined) {
  const [src, setSrc] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let disposed = false;
    let objectUrl: string | null = null;
    const reference = parseAvatarResourceReference(avatarUrl);
    const isSystemAvatar = avatarUrl === SYSTEM_AVATAR_REFERENCE;

    setSrc(null);
    setLoading(Boolean(reference || isSystemAvatar));

    if (!avatarUrl) {
      return () => undefined;
    }

    if (!reference && !isSystemAvatar) {
      setSrc(avatarUrl);
      setLoading(false);
      return () => undefined;
    }

    if (isSystemAvatar && getAuthDataSource() !== "api") {
      setLoading(false);
      return () => undefined;
    }

    if (!isSystemAvatar && getServiceDataSource("resource") !== "api") {
      setLoading(false);
      return () => undefined;
    }

    const controller = new AbortController();
    const download = isSystemAvatar
      ? downloadAccountAvatar(controller.signal)
      : downloadResourceBinary(reference!, controller.signal);
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
