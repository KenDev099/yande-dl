import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { onPostStatus, onPostsDiscovered } from "@/ipc/events";
import type { PostInfo, PostStatus } from "@/ipc/types";

export interface SubscriptionPosts {
  // jobId of the job whose posts these are. When a NEW job starts for the
  // same subscription, the post map resets — the user wants to see only the
  // current run, not a merged history.
  currentJobId: string;
  byId: Record<number, PostInfo>;
}

interface PostsByJobValue {
  bySubscription: Record<string, SubscriptionPosts>;
}

const PostsByJobContext = createContext<PostsByJobValue | null>(null);

// Lives at the AppLayout level so per-post events are captured no matter
// which page is mounted (e.g. starting a download on /subscriptions and
// then navigating to /tags/:id still shows the thumbnails).
export function PostsByJobProvider({ children }: { children: ReactNode }) {
  const [bySubscription, setBySubscription] = useState<
    Record<string, SubscriptionPosts>
  >({});

  useEffect(() => {
    const offDiscoveredP = onPostsDiscovered((e) => {
      setBySubscription((prev) => {
        const cur = prev[e.subscriptionId];
        const isNewJob = !cur || cur.currentJobId !== e.jobId;
        const merged = isNewJob ? {} : { ...cur.byId };
        for (const p of e.posts) merged[p.postId] = p;
        return {
          ...prev,
          [e.subscriptionId]: { currentJobId: e.jobId, byId: merged },
        };
      });
    });

    const offStatusP = onPostStatus((e) => {
      setBySubscription((prev) => {
        const cur = prev[e.subscriptionId];
        if (!cur || cur.currentJobId !== e.jobId) return prev;
        const existing = cur.byId[e.postId];
        if (!existing) return prev;
        return {
          ...prev,
          [e.subscriptionId]: {
            ...cur,
            byId: {
              ...cur.byId,
              [e.postId]: { ...existing, status: e.status as PostStatus },
            },
          },
        };
      });
    });

    return () => {
      offDiscoveredP.then((fn) => fn());
      offStatusP.then((fn) => fn());
    };
  }, []);

  const value = useMemo(() => ({ bySubscription }), [bySubscription]);
  return (
    <PostsByJobContext.Provider value={value}>
      {children}
    </PostsByJobContext.Provider>
  );
}

export function usePostsByJob() {
  const ctx = useContext(PostsByJobContext);
  if (!ctx) {
    throw new Error("usePostsByJob must be used inside <PostsByJobProvider>");
  }
  return ctx;
}
