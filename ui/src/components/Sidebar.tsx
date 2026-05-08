import { NavLink } from "react-router-dom";
import { Layers, Settings as SettingsIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

export function Sidebar() {
  const { t } = useTranslation();

  const navItems = [
    { to: "/subscriptions", icon: Layers, label: t("sidebar.subscriptions") },
    { to: "/settings", icon: SettingsIcon, label: t("sidebar.settings") },
  ];

  return (
    <aside className="flex h-full w-56 flex-col border-r border-border bg-surface">
      <div className="flex h-14 items-center px-5">
        <span className="text-base font-semibold tracking-tight">
          yande<span className="text-accent">-dl</span>
        </span>
      </div>
      <nav className="flex-1 space-y-1 px-2 py-2">
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
      <div className="border-t border-border px-4 py-3 text-xs text-muted-foreground">
        v0.1.0
      </div>
    </aside>
  );
}
