import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DownloadCompletedEvent,
  DownloadProgressEvent,
  NotificationEvent,
} from "./types";

export const Events = {
  downloadProgress: "download:progress",
  downloadCompleted: "download:completed",
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

export function onNotification(
  cb: (e: NotificationEvent) => void,
): Promise<UnlistenFn> {
  return listen<NotificationEvent>(Events.notification, (event) =>
    cb(event.payload),
  );
}
