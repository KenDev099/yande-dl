import { invoke } from "@tauri-apps/api/core";
import type {
  ActiveJobDto,
  ImportMode,
  ImportReport,
  Settings,
  StartDownloadResp,
  SubscriptionDto,
} from "./types";

export const ipc = {
  subscriptions: {
    list: () => invoke<SubscriptionDto[]>("list_subscriptions"),
    add: (provider: string, tag: string) =>
      invoke<SubscriptionDto>("add_subscription", { provider, tag }),
    remove: (id: string) => invoke<void>("remove_subscription", { id }),
    export: (dest: string) => invoke<void>("export_subscriptions", { dest }),
    import: (source: string, mode: ImportMode) =>
      invoke<ImportReport>("import_subscriptions", { source, mode }),
  },
  download: {
    start: (subscriptionId: string, incremental: boolean) =>
      invoke<StartDownloadResp>("start_download", {
        subscriptionId,
        incremental,
      }),
    cancel: (jobId: string) => invoke<void>("cancel_job", { jobId }),
    listActive: () => invoke<ActiveJobDto[]>("list_active_jobs"),
  },
  settings: {
    get: () => invoke<Settings>("get_settings"),
    update: (settings: Settings) =>
      invoke<Settings>("update_settings", { settings }),
  },
  system: {
    openFolder: (path?: string) => invoke<void>("open_folder", { path }),
    openPostUrl: (provider: string, postId: number) =>
      invoke<void>("open_post_url", { provider, postId }),
  },
};
