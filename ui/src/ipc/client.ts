import { invoke } from "@tauri-apps/api/core";
import type {
  ActiveJobDto,
  ImportMode,
  ImportReport,
  PreviewResp,
  Settings,
  StartBatchResp,
  StartDownloadResp,
  SubscriptionDto,
} from "./types";

export const ipc = {
  subscriptions: {
    list: () => invoke<SubscriptionDto[]>("list_subscriptions"),
    add: (provider: string, tag: string, displayName?: string | null) =>
      invoke<SubscriptionDto>("add_subscription", {
        provider,
        tag,
        displayName: displayName ?? null,
      }),
    updateDisplayName: (id: string, displayName: string | null) =>
      invoke<SubscriptionDto>("update_subscription_display_name", {
        id,
        displayName,
      }),
    remove: (id: string) => invoke<void>("remove_subscription", { id }),
    export: (dest: string) => invoke<void>("export_subscriptions", { dest }),
    import: (source: string, mode: ImportMode) =>
      invoke<ImportReport>("import_subscriptions", { source, mode }),
    countDownloaded: (subscriptionId: string) =>
      invoke<number>("count_downloaded_files", { subscriptionId }),
  },
  download: {
    start: (subscriptionId: string, incremental: boolean) =>
      invoke<StartDownloadResp>("start_download", {
        subscriptionId,
        incremental,
      }),
    startAll: (incremental: boolean) =>
      invoke<StartBatchResp>("start_download_all", { incremental }),
    cancel: (jobId: string) => invoke<void>("cancel_job", { jobId }),
    cancelAll: () => invoke<void>("cancel_all_jobs"),
    getActiveBatch: () =>
      invoke<StartBatchResp | null>("get_active_batch"),
    listActive: () => invoke<ActiveJobDto[]>("list_active_jobs"),
    preview: (subscriptionId: string, page?: number, jobId?: string) =>
      invoke<PreviewResp>("preview_subscription", {
        subscriptionId,
        page: page ?? 1,
        jobId: jobId ?? null,
      }),
    downloadSelected: (subscriptionId: string, postIds: number[]) =>
      invoke<StartDownloadResp>("download_selected_posts", {
        subscriptionId,
        postIds,
      }),
  },
  settings: {
    get: () => invoke<Settings>("get_settings"),
    update: (settings: Settings) =>
      invoke<Settings>("update_settings", { settings }),
  },
  system: {
    openFolder: (path?: string) => invoke<void>("open_folder", { path }),
    openUrl: (url: string) => invoke<void>("open_url", { url }),
    openPostUrl: (provider: string, postId: number) =>
      invoke<void>("open_post_url", { provider, postId }),
    openTagUrl: (provider: string, normalizedTag: string) =>
      invoke<void>("open_tag_url", { provider, normalizedTag }),
  },
};
