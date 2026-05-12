import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "@/ipc/client";

const KEY = ["subscriptions"];

export function useSubscriptions() {
  return useQuery({
    queryKey: KEY,
    queryFn: () => ipc.subscriptions.list(),
  });
}

export function useAddSubscription() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      provider: string;
      tag: string;
      displayName?: string | null;
    }) => ipc.subscriptions.add(input.provider, input.tag, input.displayName),
    onSuccess: () => qc.invalidateQueries({ queryKey: KEY }),
  });
}

export function useUpdateSubscriptionDisplayName() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { id: string; displayName: string | null }) =>
      ipc.subscriptions.updateDisplayName(input.id, input.displayName),
    onSuccess: () => qc.invalidateQueries({ queryKey: KEY }),
  });
}

export function useRemoveSubscription() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ipc.subscriptions.remove(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: KEY }),
  });
}

export const subscriptionsKey = KEY;
