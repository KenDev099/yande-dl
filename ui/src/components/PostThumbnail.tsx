import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Check, CircleDashed, Loader2, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { ipc } from "@/ipc/client";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { PostInfo, PostStatus } from "@/ipc/types";

interface StatusVisual {
  label: string;
  icon: typeof Check;
  badgeClass: string;
}

function visualForStatus(status: PostStatus, t: (k: string) => string): StatusVisual {
  switch (status) {
    case "queued":
      return {
        label: t("postStatus.queued"),
        icon: CircleDashed,
        badgeClass: "bg-black/65 text-white",
      };
    case "downloading":
      return {
        label: t("postStatus.downloading"),
        icon: Loader2,
        badgeClass: "bg-accent text-accent-foreground",
      };
    case "saved":
      return {
        label: t("postStatus.saved"),
        icon: Check,
        badgeClass: "bg-emerald-500 text-white",
      };
    case "skipped":
      return {
        label: t("postStatus.skipped"),
        icon: Check,
        badgeClass: "bg-slate-500/80 text-white",
      };
    case "failed":
      return {
        label: t("postStatus.failed"),
        icon: X,
        badgeClass: "bg-red-500 text-white",
      };
    case "cancelled":
      return {
        label: t("postStatus.cancelled"),
        icon: X,
        badgeClass: "bg-slate-600/80 text-white",
      };
  }
}

interface Props {
  post: PostInfo;
  provider: string;
  selected?: boolean;
  onToggleSelect?: () => void;
}

export function PostThumbnail({ post, provider, selected, onToggleSelect }: Props) {
  const { t } = useTranslation();
  const visual = visualForStatus(post.status, t);
  const Icon = visual.icon;
  const src = post.sampleUrl ?? post.previewUrl;
  const dimmed = post.status === "queued" || post.status === "cancelled";

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t("postMenu.clipboardCopied"));
    } catch (e) {
      toast.error(t("postMenu.clipboardError", { error: String(e) }));
    }
  };

  const openOriginal = async () => {
    try {
      await ipc.system.openUrl(post.originalUrl);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const openPostPage = async () => {
    try {
      await ipc.system.openPostUrl(provider, post.postId);
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          onClick={onToggleSelect}
          className={cn(
            "group relative block w-full overflow-hidden rounded-md bg-surface-2 text-left transition-opacity",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
            selected && "ring-2 ring-accent ring-offset-1 ring-offset-background",
            dimmed && !selected && "opacity-60",
          )}
          style={{ aspectRatio: "3 / 4" }}
          title={`#${post.postId} · ${visual.label}`}
        >
          <img
            src={src}
            alt={`#${post.postId}`}
            loading="lazy"
            className="h-full w-full object-cover"
            onError={(e) => {
              const img = e.currentTarget;
              if (img.src !== post.previewUrl) {
                img.src = post.previewUrl;
              } else {
                img.style.display = "none";
              }
            }}
          />

          {selected && (
            <div className="absolute left-1.5 top-1.5 flex h-5 w-5 items-center justify-center rounded-full bg-accent text-accent-foreground shadow">
              <Check className="h-3 w-3" strokeWidth={3} />
            </div>
          )}

          <div
            className={cn(
              "absolute right-1.5 top-1.5 flex items-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] font-medium",
              visual.badgeClass,
            )}
          >
            <Icon
              className={cn(
                "h-3 w-3",
                post.status === "downloading" && "animate-spin",
              )}
            />
            <span className="hidden group-hover:inline">{visual.label}</span>
          </div>

          <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-2 pb-1 pt-3 text-[10px] text-white opacity-0 transition-opacity group-hover:opacity-100">
            #{post.postId} · {post.width}×{post.height}
          </div>
        </button>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={openOriginal}>
          {t("postMenu.openOriginal")}
        </ContextMenuItem>
        <ContextMenuItem onClick={() => copy(post.originalUrl)}>
          {t("postMenu.copyOriginalUrl")}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={openPostPage}>
          {t("postMenu.openPostPage")}
        </ContextMenuItem>
        <ContextMenuItem onClick={() => copy(String(post.postId))}>
          {t("postMenu.copyPostId")}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
