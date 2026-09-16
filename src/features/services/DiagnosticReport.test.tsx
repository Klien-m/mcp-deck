import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { DiagnosticReport } from "./DiagnosticReport";

afterEach(cleanup);

describe("静态诊断的连接状态边界", () => {
  it("找到命令也始终标为未检测连接，并保留补查 PATH 的修复提示", () => {
    render(<DiagnosticReport result={{
      issues: ["仅在补充目录发现命令"],
      executable: "/custom/bin/server",
      note: "目标工具运行环境可能不同。",
      items: [
        { code: "config.valid", level: "ok", message: "配置字段有效" },
        { code: "command.fallback", level: "warning", message: "仅在补充目录发现命令", hint: "使用完整命令路径或补全 PATH。" },
      ],
    }} />);
    expect(screen.getByText("未检测连接")).toBeTruthy();
    expect(screen.getByText("1 项需要确认")).toBeTruthy();
    expect(screen.getByText("使用完整命令路径或补全 PATH。")).toBeTruthy();
    expect(screen.getByText("/custom/bin/server")).toBeTruthy();
    expect(screen.queryByText("连接成功")).toBeNull();
  });

  it("旧版结果可继续显示，不会把无问题误称为连接成功", () => {
    render(<DiagnosticReport result={{ issues: [], executable: null, note: "未启动服务。" }} />);
    expect(screen.getByText("静态检查未发现问题")).toBeTruthy();
    expect(screen.getByText("未检测连接")).toBeTruthy();
  });

  it("配置错误优先显示且包含可操作的修复建议", () => {
    render(<DiagnosticReport result={{
      issues: ["工作目录不存在"], executable: null, note: "未启动服务。",
      items: [{ code: "cwd.missing", level: "error", message: "工作目录不存在", hint: "选择已有目录。" }],
    }} />);
    expect(screen.getByText("1 项配置需要修正")).toBeTruthy();
    expect(screen.getByText("选择已有目录。")).toBeTruthy();
    expect(screen.getByText("需修正")).toBeTruthy();
  });
});
