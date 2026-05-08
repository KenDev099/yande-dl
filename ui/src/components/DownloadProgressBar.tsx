import { useTranslation } from "react-i18next";
import type { ActiveJobDto } from "@/ipc/types";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { ipc } from "@/ipc/client";
import { X } from "lucide-react";

export function DownloadProgressBar({ job }: { job: ActiveJobDto }) {
  const { t } = useTranslation();
  const total = job.fetched || 1;
  const done = job.saved + job.skipped + job.failed + job.cancelled;
  const pct = Math.min(100, Math.round((done / total) * 100));

  const labelKey = job.failed > 0
    ? "activeJobs.progressLabelFailed"
    : "activeJobs.progressLabel";

  return (
    <div className="space-y-2 rounded-md border border-border bg-surface-2 p-3">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{job.tag || "—"}</p>
          <p className="text-xs text-muted-foreground">
            {t(labelKey, {
              page: job.currentPage,
              saved: job.saved,
              skipped: job.skipped,
              failed: job.failed,
            })}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => ipc.download.cancel(job.jobId)}
          title={t("activeJobs.cancel")}
        >
          <X className="h-4 w-4" />
        </Button>
      </div>
      <Progress value={pct} />
    </div>
  );
}
