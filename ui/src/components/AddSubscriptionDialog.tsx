import { useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Plus } from "lucide-react";
import { useAddSubscription } from "@/hooks/useSubscriptions";

const PROVIDERS = [
  { id: "yande", name: "Yande.re" },
  { id: "konachan", name: "Konachan" },
];

export function AddSubscriptionDialog() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [provider, setProvider] = useState(PROVIDERS[0].id);
  const [tag, setTag] = useState("");
  const [displayName, setDisplayName] = useState("");
  const add = useAddSubscription();

  const submit = async () => {
    if (!tag.trim()) return;
    try {
      const trimmedAlias = displayName.trim();
      await add.mutateAsync({
        provider,
        tag,
        displayName: trimmedAlias.length > 0 ? trimmedAlias : null,
      });
      toast.success(t("addDialog.toastAdded", { tag: tag.trim() }));
      setTag("");
      setDisplayName("");
      setOpen(false);
    } catch (e) {
      toast.error(t("addDialog.toastError", { error: String(e) }));
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus className="mr-1 h-4 w-4" />
          {t("subscriptions.newButton")}
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("addDialog.title")}</DialogTitle>
          <DialogDescription>{t("addDialog.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label>{t("addDialog.source")}</Label>
            <RadioGroup
              value={provider}
              onValueChange={setProvider}
              className="grid grid-cols-2 gap-2"
            >
              {PROVIDERS.map((p) => (
                <Label
                  key={p.id}
                  htmlFor={`provider-${p.id}`}
                  className="flex cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-2 px-3 py-2 font-normal has-[:checked]:border-accent"
                >
                  <RadioGroupItem id={`provider-${p.id}`} value={p.id} />
                  <span>{p.name}</span>
                </Label>
              ))}
            </RadioGroup>
          </div>
          <div className="space-y-2">
            <Label htmlFor="tag-input">{t("addDialog.tag")}</Label>
            <Input
              id="tag-input"
              placeholder={t("addDialog.tagPlaceholder")}
              value={tag}
              onChange={(e) => setTag(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.nativeEvent.isComposing) submit();
              }}
              autoFocus
            />
            <p className="text-xs text-muted-foreground">
              {t("addDialog.tagHint")}
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="display-name-input">
              {t("addDialog.displayNameLabel")}
            </Label>
            <Input
              id="display-name-input"
              placeholder={t("addDialog.displayNamePlaceholder")}
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.nativeEvent.isComposing) submit();
              }}
            />
            <p className="text-xs text-muted-foreground">
              {t("addDialog.displayNameHint")}
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => setOpen(false)}
            disabled={add.isPending}
          >
            {t("common.cancel")}
          </Button>
          <Button onClick={submit} disabled={!tag.trim() || add.isPending}>
            {add.isPending ? t("addDialog.submitting") : t("common.add")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
