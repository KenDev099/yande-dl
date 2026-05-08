import { createHashRouter, Navigate } from "react-router-dom";
import { AppLayout } from "@/components/AppLayout";
import { FirstRunGate } from "@/components/FirstRunGate";
import { SubscriptionsPage } from "@/pages/Subscriptions";
import { SettingsPage } from "@/pages/Settings";

// We use a hash router so the same routing works under both `tauri dev`
// (http://localhost:5173) and bundled builds (tauri://localhost / asset://).
export const router = createHashRouter([
  {
    path: "/",
    element: (
      <FirstRunGate>
        <AppLayout />
      </FirstRunGate>
    ),
    children: [
      { index: true, element: <Navigate to="/subscriptions" replace /> },
      { path: "subscriptions", element: <SubscriptionsPage /> },
      { path: "settings", element: <SettingsPage /> },
    ],
  },
]);
