import { useEffect, useState } from "react";

import { downloadResourceBinary } from "@/domain/resource/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { parseAvatarResourceReference } from "@/shared/lib/avatarResource";

export function useManagedAvatar(avatarUrl: string | null | undefined) {
  const [src, setSrc] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let disposed = false;
    let objectUrl: string | null = null;
    const reference = parseAvatarResourceReference(avatarUrl);

    setSrc(null);
    setLoading(Boolean(reference));

    if (!avatarUrl) {
      return () => undefined;
    }

    if (!reference) {
      setSrc(avatarUrl);
      setLoading(false);
      return () => undefined;
    }

    if (getServiceDataSource("resource") !== "api") {
      setLoading(false);
      return () => undefined;
    }

    const controller = new AbortController();
    void downloadResourceBinary(reference, controller.signal)
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
