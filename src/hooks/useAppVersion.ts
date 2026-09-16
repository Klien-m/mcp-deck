import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";

/** 原生版本以安装包元数据为准，不从源码 package.json 推断发布版本。 */
export function useAppVersion() {
  const [version, setVersion] = useState("—");
  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setVersion("开发预览");
      return;
    }
    getVersion()
      .then((value) => { if (active) setVersion(value); })
      .catch(() => { if (active) setVersion("版本未知"); });
    return () => { active = false; };
  }, []);
  return version;
}
