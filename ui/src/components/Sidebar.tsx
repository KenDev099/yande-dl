import { NavLink } from "react-router-dom";
import { Layers, Settings as SettingsIcon, Tag } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { useSubscriptions } from "@/hooks/useSubscriptions";
import { usePostsByJob } from "@/hooks/usePostsByJob";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { PostInfo, SubscriptionDto } from "@/ipc/types";
import logoUrl from "@/assets/logo.svg";

export function Sidebar() {
  const { t } = useTranslation();
  const { data: subscriptions } = useSubscriptions();
  const { bySubscription } = usePostsByJob();

  const navItems = [
    { to: "/subscriptions", icon: Layers, label: t("sidebar.subscriptions") },
    { to: "/settings", icon: SettingsIcon, label: t("sidebar.settings") },
  ];

  return (
    <aside className="flex h-full w-56 flex-col border-r border-border bg-surface">
      <div className="flex h-14 shrink-0 items-center gap-2 px-4">
        <img src={logoUrl} alt="yande-dl" className="h-8 w-8" />
        <span className="text-base font-semibold tracking-tight">
          yande<span className="text-accent">-dl</span>
        </span>
      </div>
      <nav className="shrink-0 space-y-1 px-2 py-2">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-surface-2 text-foreground"
                  : "text-muted-foreground hover:bg-surface-2 hover:text-foreground",
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </NavLink>
        ))}
      </nav>

      <div className="mt-2 flex min-h-0 flex-1 flex-col border-t border-border">
        <div className="flex items-center gap-1.5 px-5 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          <Tag className="h-3 w-3" />
          <span>{t("sidebar.tagsHeading")}</span>
          {subscriptions && subscriptions.length > 0 && (
            <span className="opacity-60">· {subscriptions.length}</span>
          )}
        </div>
        {subscriptions && subscriptions.length === 0 ? (
          <p className="px-5 py-2 text-xs text-muted-foreground">
            {t("sidebar.tagsEmpty")}
          </p>
        ) : (
          <ScrollArea className="flex-1">
            <div className="space-y-0.5 px-2 pb-2">
              {subscriptions?.map((sub) => (
                <TagNavItem
                  key={sub.id}
                  sub={sub}
                  posts={bySubscription[sub.id]?.byId}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </div>

      <div className="shrink-0 border-t border-border px-4 py-3 text-xs text-muted-foreground">
        v0.1.0
      </div>
    </aside>
  );
}

function TagNavItem({
  sub,
  posts,
}: {
  sub: SubscriptionDto;
  posts: Record<number, PostInfo> | undefined;
}) {
  const primary = sub.displayName ?? sub.tag;
  const showSecondary = !!sub.displayName && sub.displayName !== sub.tag;

  const inFlight = posts
    ? Object.values(posts).filter((p) => p.status === "downloading").length
    : 0;

  return (
    <NavLink
      to={`/tags/${sub.id}`}
      className={({ isActive }) =>
        cn(
          "flex flex-col rounded-md px-3 py-1.5 transition-colors",
          isActive
            ? "bg-surface-2 text-foreground"
            : "text-muted-foreground hover:bg-surface-2 hover:text-foreground",
        )
      }
    >
      <div className="flex items-center gap-2">
        <span className="truncate text-sm font-medium">{primary}</span>
        {inFlight > 0 && (
          <span className="ml-auto shrink-0 rounded bg-accent/15 px-1.5 text-[10px] font-medium text-accent">
            {inFlight}
          </span>
        )}
      </div>
      {showSecondary && (
        <span className="mono truncate text-[11px] opacity-60">{sub.tag}</span>
      )}
    </NavLink>
  );
}
