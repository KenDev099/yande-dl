import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useUpdateSubscriptionDisplayName } from "@/hooks/useSubscriptions";
import type { SubscriptionDto } from "@/ipc/types";

interface Props {
  sub: SubscriptionDto;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function RenameSubscriptionDialog({ sub, open, onOpenChange }: Props) {
  const { t } = useTranslation();
  const [value, setValue] = useState(sub.displayName ?? "");
  const mutation = useUpdateSubscriptionDisplayName();

  // Reset input whenever the dialog opens for a (possibly different) sub.
  useEffect(() => {
    if (open) setValue(sub.displayName ?? "");
  }, [open, sub.displayName]);

  const submit = async () => {
    const trimmed = value.trim();
    try {
      await mutation.mutateAsync({
        id: sub.id,
        displayName: trimmed.length > 0 ? trimmed : null,
      });
      toast.success(t("rename.toastUpdated"));
      onOpenChange(false);
    } catch (e) {
      toast.error(t("rename.toastError", { error: String(e) }));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("rename.title")}</DialogTitle>
          <DialogDescription>{t("rename.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-2 py-2">
          <Label htmlFor="rename-input">{t("rename.label")}</Label>
          <Input
            id="rename-input"
            placeholder={t("rename.placeholder")}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.nativeEvent.isComposing) submit();
            }}
            autoFocus
          />
          <p className="mono text-xs text-muted-foreground">{sub.tag}</p>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={mutation.isPending}
          >
            {t("common.cancel")}
          </Button>
          <Button onClick={submit} disabled={mutation.isPending}>
            {mutation.isPending ? t("common.saving") : t("rename.submit")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
