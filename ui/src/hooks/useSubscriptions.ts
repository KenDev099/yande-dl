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
    mutationFn: (input: { provider: string; tag: string }) =>
      ipc.subscriptions.add(input.provider, input.tag),
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
