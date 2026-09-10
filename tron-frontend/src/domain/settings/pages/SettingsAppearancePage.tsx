import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { InfoTooltip, PageHeader, SurfaceCard, useTheme } from "@/shared";
import {
  isSystemFontSize,
  maximumSystemFontSize,
  minimumSystemFontSize,
} from "@/shared/lib/systemFontSize";
import { Input } from "@/components/ui/input";
import {
  Item,
  ItemContent,
  ItemTitle,
  ItemDescription,
  ItemActions,
} from "@/components/ui/item";

export function SettingsAppearancePage() {
  const { t: shellT } = useTranslation("shell");
  const { t: pageT } = useTranslation("pages");
  const { fontSize, setFontSize } = useTheme();
  const [draftFontSize, setDraftFontSize] = useState(String(fontSize));

  useEffect(() => {
    setDraftFontSize(String(fontSize));
  }, [fontSize]);

  function handleFontSizeChange(value: string) {
    setDraftFontSize(value);
    const nextFontSize = Number(value);
    if (isSystemFontSize(nextFontSize)) {
      setFontSize(nextFontSize);
    }
  }

  function restoreCurrentFontSize() {
    if (!isSystemFontSize(Number(draftFontSize))) {
      setDraftFontSize(String(fontSize));
    }
  }

  const tips = useMemo(() => {
    return pageT("settingsAppearance.fontSizeError", {
      min: minimumSystemFontSize,
      max: maximumSystemFontSize,
    });
  }, []);

  return (
    <div className="w-full space-y-4">
      <PageHeader title={shellT("settings.items.appearance")} />
      <SurfaceCard className="p-0">
        <Item size="xs">
          <ItemContent>
            <ItemTitle>
              {pageT("settingsAppearance.fontSize")}
              <InfoTooltip ariaLabel={tips}>{tips}</InfoTooltip>
            </ItemTitle>
            <ItemDescription>
              {pageT("settingsAppearance.fontSizeTips")}
            </ItemDescription>
          </ItemContent>
          <ItemActions>
            <Input
              className=""
              max={maximumSystemFontSize}
              min={minimumSystemFontSize}
              step={1}
              type="number"
              value={draftFontSize}
              onBlur={restoreCurrentFontSize}
              onChange={(event) => handleFontSizeChange(event.target.value)}
            />
            <ItemDescription className="flex-shrink-0">px</ItemDescription>
          </ItemActions>
        </Item>
      </SurfaceCard>
    </div>
  );
}
