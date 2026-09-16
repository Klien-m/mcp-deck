import type { Change } from "../../types";

/** 变更与历史状态的中文标签；状态键需与 IPC 协议一致，服务详情与同步弹窗共享变更文案。 */
export const actionNames = {
  add: "新增配置",
  update: "更新配置",
  remove: "移除配置",
};
export const pendingActionNames = {
  add: "待新增",
  update: "待更新",
  remove: "待移除",
};
/** 明确方向和配置键，避免将取消分配误解为复制到另一工具。 */
export function describeChange(change: Pick<Change, "action" | "targetName" | "key">) {
  if (change.action === "remove") return `将从 ${change.targetName} 移除 ${change.key}`;
  if (change.action === "add") return `将向 ${change.targetName} 新增 ${change.key}`;
  return `将更新 ${change.targetName} 中的 ${change.key}`;
}
export const historyNames = {
  applied: "已写入",
  recovered: "已恢复",
  "recovery-needed": "需要处理",
  "rolled-back": "已撤回",
  "recovery-kept": "已保留磁盘版本",
};
