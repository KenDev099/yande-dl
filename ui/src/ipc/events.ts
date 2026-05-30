import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BatchCompletedEvent,
  BatchProgressEvent,
  DownloadCompletedEvent,
  DownloadProgressEvent,
  NotificationEvent,
  PostStatusUpdateEvent,
  PostsDiscoveredEvent,
} from "./types";

export const Events = {
  downloadProgress: "download:progress",
  downloadCompleted: "download:completed",
  postsDiscovered: "download:postsDiscovered",
  postStatus: "download:postStatus",
  batchProgress: "batch:progress",
  batchCompleted: "batch:completed",
  notification: "notification",
} as const;

export function onDownloadProgress(
  cb: (e: DownloadProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>(Events.downloadProgress, (event) =>
    cb(event.payload),
  );
}

export function onDownloadCompleted(
  cb: (e: DownloadCompletedEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadCompletedEvent>(Events.downloadCompleted, (event) =>
    cb(event.payload),
  );
}

export function onPostsDiscovered(
  cb: (e: PostsDiscoveredEvent) => void,
): Promise<UnlistenFn> {
  return listen<PostsDiscoveredEvent>(Events.postsDiscovered, (event) =>
    cb(event.payload),
  );
}

export function onPostStatus(
  cb: (e: PostStatusUpdateEvent) => void,
): Promise<UnlistenFn> {
  return listen<PostStatusUpdateEvent>(Events.postStatus, (event) =>
    cb(event.payload),
  );
}

export function onNotification(
  cb: (e: NotificationEvent) => void,
): Promise<UnlistenFn> {
  return listen<NotificationEvent>(Events.notification, (event) =>
    cb(event.payload),
  );
}

export function onBatchProgress(
  cb: (e: BatchProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<BatchProgressEvent>(Events.batchProgress, (event) =>
    cb(event.payload),
  );
}

export function onBatchCompleted(
  cb: (e: BatchCompletedEvent) => void,
): Promise<UnlistenFn> {
  return listen<BatchCompletedEvent>(Events.batchCompleted, (event) =>
    cb(event.payload),
  );
}
