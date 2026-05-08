import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { BlacklistEditor } from "@/components/BlacklistEditor";
import { useSettings, useUpdateSettings } from "@/hooks/useSettings";
import type { Rating, Settings } from "@/ipc/types";
import { ipc } from "@/ipc/client";
import i18n, { SUPPORTED_LANGUAGES } from "@/i18n";

const RATING_VALUES: Rating[] = ["safe", "questionable", "explicit"];

const LANG_STORAGE_KEY = "yande-dl.lang";

export function SettingsPage() {
  const { t } = useTranslation();
  const { data, isLoading } = useSettings();
  const update = useUpdateSettings();

  const [draft, setDraft] = useState<Settings | null>(null);
  // "auto" = follow system; otherwise the explicit language tag.
  const [lang, setLang] = useState<string>(
    () => localStorage.getItem(LANG_STORAGE_KEY) ?? "auto",
  );

  useEffect(() => {
    if (data) setDraft(data);
  }, [data]);

  if (isLoading || !draft) {
    return <div className="p-6 text-muted-foreground">{t("common.loading")}</div>;
  }

  const dirty = JSON.stringify(draft) !== JSON.stringify(data);

  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") {
      setDraft({ ...draft, downloadRoot: picked });
    }
  };

  const toggleRating = (r: Rating, checked: boolean) => {
    if (checked) {
      if (draft.defaultRatings.includes(r)) return;
      setDraft({ ...draft, defaultRatings: [...draft.defaultRatings, r] });
    } else {
      if (draft.defaultRatings.length === 1) return;
      setDraft({
        ...draft,
        defaultRatings: draft.defaultRatings.filter((x) => x !== r),
      });
    }
  };

  const onLanguageChange = (next: string) => {
    setLang(next);
    if (next === "auto") {
      localStorage.removeItem(LANG_STORAGE_KEY);
      const detected = navigator.language;
      const matched =
        SUPPORTED_LANGUAGES.find((l) => l === detected) ??
        SUPPORTED_LANGUAGES.find((l) => detected.startsWith(l.split("-")[0])) ??
        "zh-TW";
      void i18n.changeLanguage(matched);
    } else {
      void i18n.changeLanguage(next);
    }
  };

  const save = async () => {
    try {
      await update.mutateAsync(draft);
      toast.success(t("settings.toastSaved"));
    } catch (e) {
      toast.error(t("settings.toastError", { error: String(e) }));
    }
  };

  return (
    <div className="mx-auto w-full max-w-3xl p-6">
      <header className="mb-6">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <p className="text-sm text-muted-foreground">
          {t("settings.description")}
        </p>
      </header>

      <section className="space-y-5">
        <div>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            {t("settings.sectionDownload")}
          </h2>
          <div className="space-y-4 rounded-md border border-border bg-card p-4">
            <div className="space-y-2">
              <Label>{t("settings.downloadFolder")}</Label>
              <div className="flex gap-2">
                <Input
                  value={draft.downloadRoot ?? ""}
                  readOnly
                  placeholder={t("settings.noFolder")}
                />
                <Button variant="outline" onClick={pickFolder}>
                  <FolderOpen className="mr-1 h-4 w-4" /> {t("common.browse")}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {t("settings.filenameHint")}
                <code className="mono ml-1">
                  {"<root>/_<provider> <tag>/<provider>_<post_id>.<ext>"}
                </code>
              </p>
            </div>

            <Separator />

            <div className="space-y-2">
              <Label>{t("settings.concurrency", { n: draft.concurrency })}</Label>
              <Slider
                min={1}
                max={8}
                step={1}
                value={[draft.concurrency]}
                onValueChange={(v) =>
                  setDraft({ ...draft, concurrency: v[0] })
                }
              />
              <p className="text-xs text-muted-foreground">
                {t("settings.concurrencyHint")}
              </p>
            </div>

            <Separator />

            <div className="space-y-2">
              <Label>
                {t("settings.minDelay", { n: draft.minDelayMs })}
              </Label>
              <Slider
                min={0}
                max={2000}
                step={50}
                value={[draft.minDelayMs]}
                onValueChange={(v) =>
                  setDraft({ ...draft, minDelayMs: v[0] })
                }
              />
            </div>
          </div>
        </div>

        <div>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            {t("settings.sectionContent")}
          </h2>
          <div className="space-y-4 rounded-md border border-border bg-card p-4">
            <div className="space-y-2">
              <Label>{t("settings.ratingFilter")}</Label>
              <div className="flex gap-4">
                {RATING_VALUES.map((r) => (
                  <Label
                    key={r}
                    className="flex items-center gap-2 font-normal"
                  >
                    <Checkbox
                      checked={draft.defaultRatings.includes(r)}
                      onCheckedChange={(c) => toggleRating(r, c === true)}
                    />
                    {t(`rating.${r}`)}
                  </Label>
                ))}
              </div>
              <p className="text-xs text-muted-foreground">
                {t("settings.ratingHint")}
              </p>
            </div>

            <Separator />

            <div className="space-y-2">
              <Label>{t("settings.blacklist")}</Label>
              <BlacklistEditor
                value={draft.blacklist}
                onChange={(blacklist) => setDraft({ ...draft, blacklist })}
              />
            </div>
          </div>
        </div>

        <div>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            {t("settings.sectionAppearance")}
          </h2>
          <div className="space-y-3 rounded-md border border-border bg-card p-4">
            <Label>{t("common.language")}</Label>
            <RadioGroup
              value={lang}
              onValueChange={onLanguageChange}
              className="grid grid-cols-2 gap-2"
            >
              <Label
                htmlFor="lang-auto"
                className="flex cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-2 px-3 py-2 font-normal has-[:checked]:border-accent"
              >
                <RadioGroupItem id="lang-auto" value="auto" />
                {t("common.languageSystem")}
              </Label>
              <Label
                htmlFor="lang-zhTW"
                className="flex cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-2 px-3 py-2 font-normal has-[:checked]:border-accent"
              >
                <RadioGroupItem id="lang-zhTW" value="zh-TW" />
                {t("common.languageZhTW")}
              </Label>
              <Label
                htmlFor="lang-en"
                className="flex cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-2 px-3 py-2 font-normal has-[:checked]:border-accent"
              >
                <RadioGroupItem id="lang-en" value="en" />
                {t("common.languageEn")}
              </Label>
              <Label
                htmlFor="lang-zhCN"
                className="flex cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-2 px-3 py-2 font-normal has-[:checked]:border-accent"
              >
                <RadioGroupItem id="lang-zhCN" value="zh-CN" />
                {t("common.languageZhCN")}
              </Label>
            </RadioGroup>
            <p className="text-xs text-muted-foreground">
              {t("common.languageHint")}
            </p>
          </div>
        </div>

        <div>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            {t("settings.sectionAbout")}
          </h2>
          <div className="space-y-3 rounded-md border border-border bg-card p-4 text-sm">
            <p>
              <span className="text-muted-foreground">
                {t("common.version")}:
              </span>{" "}
              <span className="mono">0.1.0</span>
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                ipc.system.openFolder().catch(() => {
                  /* ignored */
                });
              }}
            >
              <FolderOpen className="mr-2 h-4 w-4" />
              {t("settings.openDownloadFolder")}
            </Button>
          </div>
        </div>
      </section>

      <footer className="sticky bottom-0 mt-6 flex justify-end gap-2 border-t border-border bg-background py-3">
        <Button
          variant="outline"
          onClick={() => data && setDraft(data)}
          disabled={!dirty || update.isPending}
        >
          {t("settings.reset")}
        </Button>
        <Button onClick={save} disabled={!dirty || update.isPending}>
          {update.isPending ? t("common.saving") : t("settings.saveChanges")}
        </Button>
      </footer>
    </div>
  );
}
