import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  CheckSquare,
  Download,
  ExternalLink,
  Eye,
  FolderOpen,
  Image as ImageIcon,
  Pencil,
  Plus,
  RefreshCw,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ipc } from "@/ipc/client";
import { formatTimestamp } from "@/lib/utils";
import {
  useRemoveSubscription,
  useSubscriptions,
} from "@/hooks/useSubscriptions";
import { usePostsByJob } from "@/hooks/usePostsByJob";
import { RenameSubscriptionDialog } from "@/components/RenameSubscriptionDialog";
import { PostThumbnail } from "@/components/PostThumbnail";

export function TagDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: subscriptions, isLoading } = useSubscriptions();
  const { bySubscription } = usePostsByJob();
  const remove = useRemoveSubscription();
  const [renameOpen, setRenameOpen] = useState(false);

  // Preview pagination state: track the jobId so "next page" continues the
  // same logical preview session (the usePostsByJob hook merges by jobId).
  const [previewJobId, setPreviewJobId] = useState<string | null>(null);
  const [previewPage, setPreviewPage] = useState<number>(0);
  const [previewLoading, setPreviewLoading] = useState(false);
  // hasMore tracks whether the last fetched page came back full (== limit).
  // A non-full page == no next page (matches run_job's termination rule).
  const [previewHasMore, setPreviewHasMore] = useState<boolean>(false);

  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [downloadingSelected, setDownloadingSelected] = useState(false);

  const sub = subscriptions?.find((s) => s.id === id);

  // Disk-truth count: re-runs whenever lastRunAt changes (after any job
  // completes, including selected downloads). Replaces the legacy
  // total_downloaded counter which drifted under repeated runs.
  const { data: downloadedCount } = useQuery({
    queryKey: ["downloadedCount", sub?.id, sub?.lastRunAt],
    queryFn: () => ipc.subscriptions.countDownloaded(sub!.id),
    enabled: !!sub,
  });

  if (isLoading) {
    return (
      <div className="mx-auto w-full max-w-6xl p-6">
        <Skeleton className="h-20 w-full" />
      </div>
    );
  }

  if (!sub) {
    return (
      <div className="mx-auto w-full max-w-6xl p-6">
        <Button variant="ghost" onClick={() => navigate("/subscriptions")}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          {t("sidebar.subscriptions")}
        </Button>
        <p className="mt-6 text-center text-muted-foreground">
          {t("tagDetail.notFound")}
        </p>
      </div>
    );
  }

  const primary = sub.displayName ?? sub.tag;
  const showOriginal = !!sub.displayName && sub.displayName !== sub.tag;
  const hasBaseline = sub.lastSeenPostId > 0;
  const lastRunDisplay = sub.lastRunAt
    ? t("subscriptions.cardLastRun", { time: formatTimestamp(sub.lastRunAt) })
    : t("subscriptions.cardLastRunNever");

  const postsMap = bySubscription[sub.id]?.byId;
  const posts = postsMap
    ? Object.values(postsMap).sort((a, b) => b.postId - a.postId)
    : [];

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

  const runPreview = async (nextPage: boolean) => {
    setPreviewLoading(true);
    try {
      const page = nextPage ? previewPage + 1 : 1;
      const resp = await ipc.download.preview(
        sub.id,
        page,
        nextPage ? previewJobId ?? undefined : undefined,
      );
      setPreviewJobId(resp.jobId);
      setPreviewPage(resp.page);
      setPreviewHasMore(resp.hasMore);
    } catch (e) {
      toast.error(t("tagDetail.previewError", { error: String(e) }));
    } finally {
      setPreviewLoading(false);
    }
  };

  const toggleSelect = (postId: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(postId)) next.delete(postId);
      else next.add(postId);
      return next;
    });
  };

  const clearSelection = () => setSelected(new Set());

  const toggleSelectAll = () => {
    if (selected.size === posts.length && posts.length > 0) {
      setSelected(new Set());
    } else {
      setSelected(new Set(posts.map((p) => p.postId)));
    }
  };

  const openTagPage = async () => {
    try {
      await ipc.system.openTagUrl(sub.provider, sub.normalizedTag);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const downloadSelected = async () => {
    if (selected.size === 0) return;
    setDownloadingSelected(true);
    try {
      await ipc.download.downloadSelected(sub.id, Array.from(selected));
      toast.success(t("tagDetail.toastStartingSelected", { count: selected.size }));
      clearSelection();
    } catch (e) {
      const msg = String(e);
      if (msg.includes("preview-cache miss")) {
        toast.error(t("preview.cacheMiss"));
      } else {
        toast.error(t("tagDetail.toastErrorSelected", { error: msg }));
      }
    } finally {
      setDownloadingSelected(false);
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
      navigate("/subscriptions");
    } catch (e) {
      toast.error(t("subscriptions.toastErrorRemove", { error: String(e) }));
    }
  };

  return (
    <div className="mx-auto w-full max-w-6xl p-6 pb-24">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div className="min-w-0">
          <Link
            to="/subscriptions"
            className="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <ArrowLeft className="h-3 w-3" />
            {t("sidebar.subscriptions")}
          </Link>
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded bg-surface-2 px-1.5 py-0.5 text-xs font-medium text-muted-foreground">
              {sub.providerDisplayName}
            </span>
            <h1 className="text-2xl font-semibold leading-tight">{primary}</h1>
            {showOriginal && (
              <span className="mono text-sm text-muted-foreground">
                · {sub.tag}
              </span>
            )}
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("subscriptions.cardDownloaded", {
              count: downloadedCount ?? 0,
            })}{" "}
            · {lastRunDisplay}
            {posts.length > 0 && (
              <>
                {" · "}
                {t("tagDetail.currentJobLabel", { count: posts.length })}
              </>
            )}
            {previewJobId !== null && previewPage > 0 && (
              <>
                {" · "}
                {t("tagDetail.previewPageInfo", { page: previewPage })}
                {!previewHasMore && (
                  <> · {t("tagDetail.endOfPages")}</>
                )}
              </>
            )}
          </p>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-1">
          <Button
            size="sm"
            variant="outline"
            onClick={() => runPreview(false)}
            disabled={previewLoading}
            title={t("tagDetail.previewTooltip")}
          >
            <Eye className="mr-1 h-3.5 w-3.5" />
            {previewLoading
              ? t("tagDetail.previewLoading")
              : t("tagDetail.preview")}
          </Button>
          <Button
            size="sm"
            onClick={() => startDownload(false)}
            title={t("tagDetail.downloadAllTooltip")}
          >
            <Download className="mr-1 h-3.5 w-3.5" />
            {t("tagDetail.downloadAll")}
          </Button>
          {hasBaseline && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => startDownload(true)}
              title={t("subscriptions.cardUpdateTooltip")}
            >
              <RefreshCw className="mr-1 h-3.5 w-3.5" />
              {t("subscriptions.cardUpdate")}
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
            onClick={openTagPage}
            title={t("tagDetail.openTagPage")}
          >
            <ExternalLink className="h-4 w-4" />
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
      </header>

      {posts.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-20 text-center">
          <ImageIcon className="mb-3 h-10 w-10 text-muted-foreground" />
          <p className="text-base font-medium">{t("tagDetail.emptyTitle")}</p>
          <p className="mt-1 max-w-sm text-sm text-muted-foreground">
            {t("tagDetail.emptyHint")}
          </p>
        </div>
      ) : (
        <>
          <div className="mb-3 flex items-center justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={toggleSelectAll}
            >
              {selected.size === posts.length ? (
                <>
                  <Square className="mr-1 h-3.5 w-3.5" />
                  {t("tagDetail.deselectAll")}
                </>
              ) : (
                <>
                  <CheckSquare className="mr-1 h-3.5 w-3.5" />
                  {t("tagDetail.selectAll", { count: posts.length })}
                </>
              )}
            </Button>
          </div>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {posts.map((p) => (
              <PostThumbnail
                key={p.postId}
                post={p}
                provider={sub.provider}
                selected={selected.has(p.postId)}
                onToggleSelect={() => toggleSelect(p.postId)}
              />
            ))}
          </div>
          {previewJobId !== null && (
            <div className="mt-4 flex justify-center">
              <Button
                variant="outline"
                size="sm"
                onClick={() => runPreview(true)}
                disabled={previewLoading || !previewHasMore}
              >
                <Plus className="mr-1 h-3.5 w-3.5" />
                {previewLoading
                  ? t("tagDetail.previewLoading")
                  : !previewHasMore
                    ? t("tagDetail.endOfPages")
                    : t("tagDetail.loadMore", { next: previewPage + 1 })}
              </Button>
            </div>
          )}
        </>
      )}

      {selected.size > 0 && (
        <div className="fixed bottom-4 left-1/2 z-40 -translate-x-1/2">
          <div className="flex items-center gap-2 rounded-full border border-border bg-card px-3 py-2 shadow-xl">
            <span className="text-sm">
              {t("tagDetail.selectionCount", { count: selected.size })}
            </span>
            <Button
              size="sm"
              onClick={downloadSelected}
              disabled={downloadingSelected}
            >
              <Download className="mr-1 h-3.5 w-3.5" />
              {downloadingSelected
                ? t("tagDetail.downloadSelectedLoading")
                : t("tagDetail.downloadSelected", { count: selected.size })}
            </Button>
            <Button
              size="icon"
              variant="ghost"
              onClick={clearSelection}
              title={t("tagDetail.clearSelection")}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}

      <RenameSubscriptionDialog
        sub={sub}
        open={renameOpen}
        onOpenChange={setRenameOpen}
      />
    </div>
  );
}
