import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Button } from "@/components/ui/button";
import { ChevronUp, Download } from "lucide-react";
import type { ActiveJobDto } from "@/ipc/types";
import { DownloadProgressBar } from "@/components/DownloadProgressBar";

export function ActiveJobsDrawer({ jobs }: { jobs: ActiveJobDto[] }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (jobs.length > 0) setOpen(true);
    else setOpen(false);
  }, [jobs.length]);

  if (jobs.length === 0) return null;

  return (
    <>
      <div className="pointer-events-none fixed bottom-3 right-4 z-40">
        <Button
          className="pointer-events-auto shadow-lg"
          onClick={() => setOpen(true)}
        >
          <Download className="mr-2 h-4 w-4" />
          {t("activeJobs.trigger", { count: jobs.length })}
          <ChevronUp className="ml-2 h-4 w-4" />
        </Button>
      </div>
      <Drawer open={open} onOpenChange={setOpen}>
        <DrawerContent className="max-h-[70vh]">
          <DrawerHeader>
            <DrawerTitle>{t("activeJobs.drawerTitle")}</DrawerTitle>
            <DrawerDescription>
              {t("activeJobs.drawerCount", { count: jobs.length })}
            </DrawerDescription>
          </DrawerHeader>
          <div className="space-y-2 px-4 pb-6">
            {jobs.map((j) => (
              <DownloadProgressBar key={j.jobId} job={j} />
            ))}
          </div>
        </DrawerContent>
      </Drawer>
    </>
  );
}
