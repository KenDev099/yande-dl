export interface SubscriptionDto {
  id: string;
  provider: string;
  providerDisplayName: string;
  tag: string;
  normalizedTag: string;
  lastRunAt: number | null;
  lastSeenPostId: number;
  totalDownloaded: number;
  createdAt: number;
}

export interface ActiveJobDto {
  jobId: string;
  subscriptionId: string;
  tag: string;
  currentPage: number;
  fetched: number;
  saved: number;
  skipped: number;
  failed: number;
  cancelled: number;
}

export interface DownloadProgressEvent {
  jobId: string;
  subscriptionId: string;
  currentPage: number;
  fetched: number;
  saved: number;
  skipped: number;
  failed: number;
  cancelled: number;
}

export interface DownloadCompletedEvent {
  jobId: string;
  subscriptionId: string;
  totalSaved: number;
  totalSkipped: number;
  totalFailed: number;
  totalCancelled: number;
  safeLastPostId: number;
}

export interface NotificationEvent {
  kind: "info" | "success" | "warning" | "error";
  message: string;
}

export type Rating = "safe" | "questionable" | "explicit";

export interface Settings {
  version: number;
  downloadRoot: string | null;
  concurrency: number;
  minDelayMs: number;
  defaultRatings: Rating[];
  theme: "dark" | "light" | "system";
  ageConfirmed: boolean;
  blacklist: string[];
}

export interface StartDownloadResp {
  jobId: string;
}

export type ImportMode = "replace" | "merge";

export interface ImportReport {
  added: number;
  skipped: number;
  removed: number;
}
