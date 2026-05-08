import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { Download as DownloadIcon, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Label } from "@/components/ui/label";
import { ipc } from "@/ipc/client";
import { useQueryClient } from "@tanstack/react-query";
import { subscriptionsKey } from "@/hooks/useSubscriptions";
import type { ImportMode } from "@/ipc/types";

export function ImportExportMenu() {
  const { t } = useTranslation();
  const [importOpen, setImportOpen] = useState(false);
  const [importPath, setImportPath] = useState<string | null>(null);
  const [mode, setMode] = useState<ImportMode>("merge");
  const qc = useQueryClient();

  const onExport = async () => {
    const dest = await save({
      defaultPath: "tags.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!dest) return;
    try {
      await ipc.subscriptions.export(dest);
      toast.success(t("importExport.toastExported", { path: dest }));
    } catch (e) {
      toast.error(t("importExport.toastExportFailed", { error: String(e) }));
    }
  };

  const pickImportFile = async () => {
    const picked = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof picked === "string") setImportPath(picked);
  };

  const runImport = async () => {
    if (!importPath) return;
    try {
      const report = await ipc.subscriptions.import(importPath, mode);
      qc.invalidateQueries({ queryKey: subscriptionsKey });
      let summary = t("importExport.summaryAddedSkipped", {
        added: report.added,
        skipped: report.skipped,
      });
      if (report.removed > 0) {
        summary += t("importExport.summaryRemoved", { removed: report.removed });
      }
      toast.success(t("importExport.toastImported", { summary }));
      setImportOpen(false);
      setImportPath(null);
    } catch (e) {
      toast.error(t("importExport.toastImportFailed", { error: String(e) }));
    }
  };

  return (
    <>
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={onExport}>
          <DownloadIcon className="mr-1 h-4 w-4" />{" "}
          {t("importExport.exportButton")}
        </Button>
        <Button variant="outline" size="sm" onClick={() => setImportOpen(true)}>
          <Upload className="mr-1 h-4 w-4" /> {t("importExport.importButton")}
        </Button>
      </div>

      <Dialog open={importOpen} onOpenChange={setImportOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("importExport.title")}</DialogTitle>
            <DialogDescription>
              {t("importExport.description")}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label>{t("importExport.file")}</Label>
              <Button
                variant="outline"
                className="w-full justify-start font-normal"
                onClick={pickImportFile}
              >
                <Upload className="mr-2 h-4 w-4" />
                <span className="truncate">
                  {importPath ?? t("importExport.chooseFile")}
                </span>
              </Button>
            </div>
            <div className="space-y-2">
              <Label>{t("importExport.mode")}</Label>
              <RadioGroup
                value={mode}
                onValueChange={(v) => setMode(v as ImportMode)}
              >
                <div className="flex items-start gap-2">
                  <RadioGroupItem value="merge" id="merge" />
                  <Label htmlFor="merge" className="font-normal">
                    {t("importExport.modeMerge")}
                  </Label>
                </div>
                <div className="flex items-start gap-2">
                  <RadioGroupItem value="replace" id="replace" />
                  <Label htmlFor="replace" className="font-normal">
                    {t("importExport.modeReplace")}
                  </Label>
                </div>
              </RadioGroup>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setImportOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={runImport} disabled={!importPath}>
              {t("importExport.submit")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
