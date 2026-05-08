import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "@/ipc/client";
import type { Settings } from "@/ipc/types";

const KEY = ["settings"];

export function useSettings() {
  return useQuery({
    queryKey: KEY,
    queryFn: () => ipc.settings.get(),
  });
}

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: Settings) => ipc.settings.update(settings),
    onSuccess: (data) => {
      qc.setQueryData(KEY, data);
    },
  });
}
