import { Layers, RefreshCw, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useEffect } from "react";
import { useSubscriptions } from "@/hooks/useSubscriptions";
import { useBatchProgress } from "@/hooks/useBatchProgress";
import { AddSubscriptionDialog } from "@/components/AddSubscriptionDialog";
import { SubscriptionCard } from "@/components/SubscriptionCard";
import { ImportExportMenu } from "@/components/ImportExportMenu";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { ipc } from "@/ipc/client";
import { onBatchCompleted } from "@/ipc/events";

export function SubscriptionsPage() {
  const { t } = useTranslation();
  const { data, isLoading } = useSubscriptions();
  const batch = useBatchProgress();

  useEffect(() => {
    const p = onBatchCompleted((e) => {
      if (e.cancelled) {
        toast(
          t("subscriptions.toastBatchCancelled", {
            processed: e.processed,
            total: e.total,
          }),
        );
      } else {
        toast.success(
          t("subscriptions.toastBatchDone", {
            processed: e.processed,
            total: e.total,
          }),
        );
      }
    });
    return () => {
      p.then((u) => u());
    };
  }, [t]);

  const startBatch = async () => {
    if (!data || data.length === 0) return;
    try {
      const resp = await ipc.download.startAll(true);
      toast(t("subscriptions.toastStartingBatch", { total: resp.total }));
    } catch (e) {
      toast.error(t("subscriptions.toastErrorStartBatch", { error: String(e) }));
    }
  };

  const stopBatch = async () => {
    try {
      await ipc.download.cancelAll();
    } catch (e) {
      // cancel_all_jobs is idempotent and shouldn't fail, but surface anything.
      toast.error(String(e));
    }
  };

  return (
    <div className="mx-auto w-full max-w-4xl p-6">
      <header className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">{t("subscriptions.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {t("subscriptions.description")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {batch ? (
            <Button variant="destructive" size="sm" onClick={stopBatch}>
              <Square className="mr-1 h-4 w-4" />
              {t("subscriptions.stopAll", {
                current: batch.currentIndex + 1,
                total: batch.total,
              })}
            </Button>
          ) : (
            (data?.length ?? 0) > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={startBatch}
                title={t("subscriptions.updateAllTooltip")}
              >
                <RefreshCw className="mr-1 h-4 w-4" />
                {t("subscriptions.updateAll")}
              </Button>
            )
          )}
          <ImportExportMenu />
          <AddSubscriptionDialog />
        </div>
      </header>

      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-16 w-full" />
          ))}
        </div>
      ) : (data?.length ?? 0) === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-16 text-center">
          <Layers className="mb-3 h-10 w-10 text-muted-foreground" />
          <p className="text-base font-medium">
            {t("subscriptions.emptyTitle")}
          </p>
          <p className="mt-1 max-w-sm text-sm text-muted-foreground">
            {t("subscriptions.emptyHint")}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {data!.map((s) => (
            <SubscriptionCard key={s.id} sub={s} />
          ))}
        </div>
      )}
    </div>
  );
}
