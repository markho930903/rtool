import { useEffect } from "react";
import { Outlet, useLocation, useNavigate, useSearchParams } from "react-router";

import { normalizeToolId } from "@/components/tools/tool-registry";

export default function ToolsPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams] = useSearchParams();

  useEffect(() => {
    const isToolsRoot = location.pathname === "/tools" || location.pathname === "/tools/";
    if (!isToolsRoot) {
      return;
    }

    const normalizedToolId = normalizeToolId(searchParams.get("tool"));
    if (!normalizedToolId) {
      return;
    }

    const nextSearchParams = new URLSearchParams(searchParams);
    nextSearchParams.delete("tool");
    const nextSearch = nextSearchParams.toString();

    void navigate(
      {
        pathname: `/tools/${normalizedToolId}`,
        search: nextSearch ? `?${nextSearch}` : "",
      },
      { replace: true },
    );
  }, [location.pathname, navigate, searchParams]);

  return <Outlet />;
}
