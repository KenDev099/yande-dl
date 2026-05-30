import { useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import {
  Download,
  FolderOpen,
  Pencil,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { Link } from "react-router-dom";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ipc } from "@/ipc/client";
import { formatTimestamp } from "@/lib/utils";
import { useRemoveSubscription } from "@/hooks/useSubscriptions";
import { RenameSubscriptionDialog } from "@/components/RenameSubscriptionDialog";
import type { SubscriptionDto } from "@/ipc/types";

export function SubscriptionCard({ sub }: { sub: SubscriptionDto }) {
  const { t } = useTranslation();
  const remove = useRemoveSubscription();
  const [renameOpen, setRenameOpen] = useState(false);

  // Disk-truth count. Re-fetches whenever lastRunAt changes (job completion
  // bumps it via touch_last_run_at / update_after_run).
  const { data: downloadedCount } = useQuery({
    queryKey: ["downloadedCount", sub.id, sub.lastRunAt],
    queryFn: () => ipc.subscriptions.countDownloaded(sub.id),
  });

  const startDownload = async (incremental: boolean) => {
    try {
      await ipc.download.start(sub.id, incremental);
      toast(
        incremental
          ? t("subscriptions.toastStartingUpdate", { tag: sub.tag })
          : t("subscriptions.toastStartingFull", { tag: sub.tag }),
      );
    } catch (e) {
      toast.error(t("subscriptions.toastErrorStart", { error: String(e) }));
    }
  };

  const openFolder = async () => {
    try {
      await ipc.system.openFolder();
    } catch (e) {
      toast.error(
        t("subscriptions.toastErrorOpenFolder", { error: String(e) }),
      );
    }
  };

  const onRemove = async () => {
    if (!confirm(t("subscriptions.removeConfirm", { tag: sub.tag }))) return;
    try {
      await remove.mutateAsync(sub.id);
      toast(t("subscriptions.toastRemoved", { tag: sub.tag }));
    } catch (e) {
      toast.error(t("subscriptions.toastErrorRemove", { error: String(e) }));
    }
  };

  const hasBaseline = sub.lastSeenPostId > 0;
  const lastRunDisplay = sub.lastRunAt
    ? t("subscriptions.cardLastRun", { time: formatTimestamp(sub.lastRunAt) })
    : t("subscriptions.cardLastRunNever");

  const primary = sub.displayName ?? sub.tag;
  const showOriginal = !!sub.displayName && sub.displayName !== sub.tag;

  return (
    <>
      <Card>
        <CardContent className="flex items-center justify-between gap-4 p-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="rounded bg-surface-2 px-1.5 py-0.5 text-xs font-medium text-muted-foreground">
                {sub.providerDisplayName}
              </span>
              <Link
                to={`/tags/${sub.id}`}
                className="min-w-0 truncate text-sm font-medium hover:underline"
              >
                {primary}
              </Link>
              {showOriginal && (
                <span className="mono shrink-0 text-xs text-muted-foreground">
                  · {sub.tag}
                </span>
              )}
            </div>
            <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
              <span>
                {t("subscriptions.cardDownloaded", {
                  count: downloadedCount ?? 0,
                })}
              </span>
              <span aria-hidden>·</span>
              <span>{lastRunDisplay}</span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            {hasBaseline ? (
              <Button
                size="sm"
                onClick={() => startDownload(true)}
                title={t("subscriptions.cardUpdateTooltip")}
              >
                <RefreshCw className="mr-1 h-3.5 w-3.5" />{" "}
                {t("subscriptions.cardUpdate")}
              </Button>
            ) : (
              <Button size="sm" onClick={() => startDownload(false)}>
                <Download className="mr-1 h-3.5 w-3.5" />{" "}
                {t("subscriptions.cardDownload")}
              </Button>
            )}
            {hasBaseline && (
              <Button
                size="icon"
                variant="ghost"
                onClick={() => startDownload(false)}
                title={t("subscriptions.cardRefetch")}
              >
                <Download className="h-4 w-4" />
              </Button>
            )}
            <Button
              size="icon"
              variant="ghost"
              onClick={() => setRenameOpen(true)}
              title={t("subscriptions.cardRename")}
            >
              <Pencil className="h-4 w-4" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              onClick={openFolder}
              title={t("subscriptions.cardOpenFolder")}
            >
              <FolderOpen className="h-4 w-4" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              onClick={onRemove}
              title={t("subscriptions.cardRemove")}
              disabled={remove.isPending}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </CardContent>
      </Card>
      <RenameSubscriptionDialog
        sub={sub}
        open={renameOpen}
        onOpenChange={setRenameOpen}
      />
    </>
  );
}
