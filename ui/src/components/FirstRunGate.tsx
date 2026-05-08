import { useSettings } from "@/hooks/useSettings";
import { FirstRunModal } from "@/components/FirstRunModal";
import { Skeleton } from "@/components/ui/skeleton";

export function FirstRunGate({ children }: { children: React.ReactNode }) {
  const { data, isLoading } = useSettings();

  if (isLoading || !data) {
    return (
      <div className="flex h-screen items-center justify-center">
        <Skeleton className="h-12 w-64" />
      </div>
    );
  }

  const needsFirstRun = !data.ageConfirmed || data.downloadRoot === null;

  return (
    <>
      {children}
      {needsFirstRun && <FirstRunModal currentSettings={data} />}
    </>
  );
}
