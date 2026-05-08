import { Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSubscriptions } from "@/hooks/useSubscriptions";
import { AddSubscriptionDialog } from "@/components/AddSubscriptionDialog";
import { SubscriptionCard } from "@/components/SubscriptionCard";
import { ImportExportMenu } from "@/components/ImportExportMenu";
import { Skeleton } from "@/components/ui/skeleton";

export function SubscriptionsPage() {
  const { t } = useTranslation();
  const { data, isLoading } = useSubscriptions();

  return (
    <div className="mx-auto w-full max-w-4xl p-6">
      <header className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">{t("subscriptions.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {t("subscriptions.description")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <ImportExportMenu />
          <AddSubscriptionDialog />
        </div>
      </header>

      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-16 w-full" />
          ))}
        </div>
      ) : (data?.length ?? 0) === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-16 text-center">
          <Layers className="mb-3 h-10 w-10 text-muted-foreground" />
          <p className="text-base font-medium">
            {t("subscriptions.emptyTitle")}
          </p>
          <p className="mt-1 max-w-sm text-sm text-muted-foreground">
            {t("subscriptions.emptyHint")}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {data!.map((s) => (
            <SubscriptionCard key={s.id} sub={s} />
          ))}
        </div>
      )}
    </div>
  );
}
