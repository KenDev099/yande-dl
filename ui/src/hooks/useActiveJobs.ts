import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import i18n from "@/i18n";
import { ipc } from "@/ipc/client";
import {
  onDownloadCompleted,
  onDownloadProgress,
  onNotification,
} from "@/ipc/events";
import type { ActiveJobDto } from "@/ipc/types";
import { subscriptionsKey } from "@/hooks/useSubscriptions";

export function useActiveJobs() {
  const [jobs, setJobs] = useState<Record<string, ActiveJobDto>>({});
  const qc = useQueryClient();

  const initial = useQuery({
    queryKey: ["active-jobs"],
    queryFn: () => ipc.download.listActive(),
    staleTime: 0,
  });

  useEffect(() => {
    if (initial.data) {
      const map: Record<string, ActiveJobDto> = {};
      for (const j of initial.data) map[j.jobId] = j;
      setJobs(map);
    }
  }, [initial.data]);

  useEffect(() => {
    const unlistenProgressP = onDownloadProgress((e) => {
      setJobs((prev) => ({
        ...prev,
        [e.jobId]: {
          jobId: e.jobId,
          subscriptionId: e.subscriptionId,
          tag: prev[e.jobId]?.tag ?? "",
          currentPage: e.currentPage,
          fetched: e.fetched,
          saved: e.saved,
          skipped: e.skipped,
          failed: e.failed,
          cancelled: e.cancelled,
        },
      }));
    });

    const unlistenCompletedP = onDownloadCompleted((e) => {
      setJobs((prev) => {
        const next = { ...prev };
        delete next[e.jobId];
        return next;
      });
      qc.invalidateQueries({ queryKey: subscriptionsKey });

      const cancelled = e.totalCancelled > 0 && e.totalSaved === 0;
      const summary =
        e.totalFailed > 0
          ? i18n.t("downloadResult.summaryWithFailures", {
              saved: e.totalSaved,
              skipped: e.totalSkipped,
              failed: e.totalFailed,
            })
          : i18n.t("downloadResult.summaryClean", {
              saved: e.totalSaved,
              skipped: e.totalSkipped,
            });

      if (cancelled) {
        toast(i18n.t("downloadResult.cancelled"));
      } else if (e.totalFailed > 0) {
        toast.warning(i18n.t("downloadResult.finished", { summary }));
      } else {
        toast.success(i18n.t("downloadResult.finished", { summary }));
      }
    });

    const unlistenNotifP = onNotification((e) => {
      switch (e.kind) {
        case "error":
          toast.error(e.message);
          break;
        case "warning":
          toast.warning(e.message);
          break;
        case "success":
          toast.success(e.message);
          break;
        default:
          toast(e.message);
      }
    });

    return () => {
      unlistenProgressP.then((fn) => fn());
      unlistenCompletedP.then((fn) => fn());
      unlistenNotifP.then((fn) => fn());
    };
  }, [qc]);

  return { jobs };
}
