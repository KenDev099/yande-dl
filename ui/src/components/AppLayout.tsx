import { Outlet } from "react-router-dom";
import { Sidebar } from "@/components/Sidebar";
import { ActiveJobsDrawer } from "@/components/ActiveJobsDrawer";
import { useActiveJobs } from "@/hooks/useActiveJobs";
import { PostsByJobProvider } from "@/hooks/usePostsByJob";

export function AppLayout() {
  const { jobs } = useActiveJobs();
  const jobList = Object.values(jobs);

  return (
    <PostsByJobProvider>
      <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
        <Sidebar />
        <main className="flex-1 overflow-y-auto">
          <Outlet />
        </main>
        <ActiveJobsDrawer jobs={jobList} />
      </div>
    </PostsByJobProvider>
  );
}
