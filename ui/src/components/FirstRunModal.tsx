import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { FolderOpen, ShieldAlert } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { useUpdateSettings } from "@/hooks/useSettings";
import type { Rating, Settings } from "@/ipc/types";

const RATING_PRESETS: Record<string, Rating[]> = {
  safe: ["safe"],
  "safe+questionable": ["safe", "questionable"],
  all: ["safe", "questionable", "explicit"],
};

export function FirstRunModal({
  currentSettings,
}: {
  currentSettings: Settings;
}) {
  const { t } = useTranslation();
  const [downloadRoot, setDownloadRoot] = useState<string | null>(
    currentSettings.downloadRoot,
  );
  const [preset, setPreset] = useState<string>("safe");
  const [confirmed, setConfirmed] = useState(currentSettings.ageConfirmed);
  const update = useUpdateSettings();

  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") setDownloadRoot(picked);
  };

  const onSave = async () => {
    if (!downloadRoot || !confirmed) return;
    await update.mutateAsync({
      ...currentSettings,
      downloadRoot,
      defaultRatings: RATING_PRESETS[preset],
      ageConfirmed: confirmed,
    });
  };

  const canSave = !!downloadRoot && confirmed;

  return (
    <Dialog open={true}>
      <DialogContent hideClose className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("firstRun.title")}</DialogTitle>
          <DialogDescription>{t("firstRun.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-5 py-2">
          <div className="space-y-2">
            <Label>{t("firstRun.downloadFolder")}</Label>
            <Button
              variant="outline"
              className="w-full justify-start font-normal"
              onClick={pickFolder}
            >
              <FolderOpen className="mr-2 h-4 w-4" />
              <span className="truncate text-left">
                {downloadRoot ?? t("firstRun.chooseFolder")}
              </span>
            </Button>
          </div>

          <div className="space-y-2">
            <Label>{t("firstRun.rating")}</Label>
            <RadioGroup value={preset} onValueChange={setPreset}>
              <div className="flex items-center gap-2">
                <RadioGroupItem value="safe" id="r-safe" />
                <Label htmlFor="r-safe" className="font-normal">
                  {t("firstRun.ratingSafe")}
                </Label>
              </div>
              <div className="flex items-center gap-2">
                <RadioGroupItem value="safe+questionable" id="r-sq" />
                <Label htmlFor="r-sq" className="font-normal">
                  {t("firstRun.ratingSafeQuestionable")}
                </Label>
              </div>
              <div className="flex items-center gap-2">
                <RadioGroupItem value="all" id="r-all" />
                <Label htmlFor="r-all" className="font-normal">
                  {t("firstRun.ratingAll")}
                </Label>
              </div>
            </RadioGroup>
          </div>

          <div className="flex items-start gap-3 rounded-md border border-border bg-surface-2 p-3">
            <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
            <div className="space-y-2 text-sm">
              <p className="text-muted-foreground">{t("firstRun.warning")}</p>
              <div className="flex items-center gap-2">
                <Checkbox
                  id="confirm"
                  checked={confirmed}
                  onCheckedChange={(c) => setConfirmed(c === true)}
                />
                <Label htmlFor="confirm" className="font-normal">
                  {t("firstRun.confirm")}
                </Label>
              </div>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button onClick={onSave} disabled={!canSave || update.isPending}>
            {update.isPending ? t("common.saving") : t("firstRun.submit")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
