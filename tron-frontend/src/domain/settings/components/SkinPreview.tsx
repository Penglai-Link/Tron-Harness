import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "@/components/ui/dialog";

/** Real shared controls make the current skin's states inspectable by its author. */
export function SkinPreview() {
  const { t } = useTranslation("pages");
  const [selected, setSelected] = useState(2);
  const label = (key: string) => t(`settingsAppearance.preview.${key}`);
  return <details className="border-t border-[var(--color-outline-variant)] pt-3">
    <summary className="cursor-pointer text-sm font-medium">{label("title")}</summary>
    <div className="mt-4 space-y-4" data-testid="skin-component-preview">
      <div className="flex flex-wrap gap-2">
        {(["default", "secondary", "outline", "ghost", "destructive", "link"] as const).map((variant) =>
          <Button key={variant} variant={variant}>{label(variant)}</Button>)}
        <Button disabled>{label("disabled")}</Button>
        <Dialog><DialogTrigger render={<Button variant="outline" />}>{label("dialog")}</DialogTrigger>
          <DialogContent><DialogTitle>{label("dialog")}</DialogTitle><Input aria-label={label("input")} placeholder={label("input")} /></DialogContent>
        </Dialog>
      </div>
      <div className="grid gap-3 sm:grid-cols-3">
        <Input aria-label={label("input")} placeholder={label("input")} />
        <Input disabled aria-label={label("disabled")} placeholder={label("disabled")} />
        <Input aria-invalid="true" aria-label={label("invalid")} placeholder={label("invalid")} />
      </div>
      <Table>
        <TableHeader><TableRow><TableHead>{label("name")}</TableHead><TableHead>{label("state")}</TableHead></TableRow></TableHeader>
        <TableBody>{[0, 1, 2].map((index) => <TableRow key={index} tabIndex={0}
          data-state={selected === index ? "selected" : undefined} aria-selected={selected === index}
          onClick={() => setSelected(index)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setSelected(index); } }}>
          <TableCell>{label("row")} {index + 1}</TableCell>
          <TableCell style={{ position: "sticky", right: 0, backgroundColor: "var(--_tron-row-bg)" }}>{label(selected === index ? "selected" : "normal")}</TableCell>
        </TableRow>)}</TableBody>
      </Table>
    </div>
  </details>;
}
