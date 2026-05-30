import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/ipc/client";
import { onBatchCompleted, onBatchProgress } from "@/ipc/events";

export interface BatchState {
  batchId: string;
  currentIndex: number;
  total: number;
  currentSubscriptionId: string | null;
}

export function useBatchProgress() {
  const [batch, setBatch] = useState<BatchState | null>(null);

  // Recover state on mount in case the page reloaded mid-batch.
  const initial = useQuery({
    queryKey: ["active-batch"],
    queryFn: () => ipc.download.getActiveBatch(),
    staleTime: 0,
  });

  useEffect(() => {
    if (initial.data) {
      setBatch({
        batchId: initial.data.batchId,
        currentIndex: 0,
        total: initial.data.total,
        currentSubscriptionId: null,
      });
    } else if (initial.data === null) {
      setBatch(null);
    }
  }, [initial.data]);

  useEffect(() => {
    const unlistenProgressP = onBatchProgress((e) => {
      setBatch({
        batchId: e.batchId,
        currentIndex: e.currentIndex,
        total: e.total,
        currentSubscriptionId: e.currentSubscriptionId,
      });
    });

    const unlistenCompletedP = onBatchCompleted(() => {
      setBatch(null);
    });

    return () => {
      unlistenProgressP.then((u) => u());
      unlistenCompletedP.then((u) => u());
    };
  }, []);

  return batch;
}
