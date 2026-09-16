import { CheckCircle2, CircleAlert, Info } from "lucide-react";
import type { Checks } from "../../types";
import "./diagnostics.css";

/** 静态检查与连接状态分开展示；兼容旧后端的纯文本问题列表。 */
export function DiagnosticReport({ result }: { result: Checks }) {
  const items = result.items ?? result.issues.map((message, index) => ({
    code: `legacy-${index}`,
    level: "warning" as const,
    message,
    hint: undefined,
  }));
  const errors = items.filter((item) => item.level === "error").length;
  const warnings = items.filter((item) => item.level === "warning").length;
  return (
    <section className="diagnostic-report" aria-label="静态检查结果">
      <div className="diagnostic-summary">
        <div>
          <strong>{errors ? `${errors} 项配置需要修正` : warnings ? `${warnings} 项需要确认` : "静态检查未发现问题"}</strong>
          <p>检查范围：配置字段、命令位置与本机文件</p>
        </div>
        <span className="diagnostic-connection"><Info size={14} aria-hidden="true" />未检测连接</span>
      </div>
      <ul className="diagnostic-items">
        {items.map((item, index) => {
          const Icon = item.level === "ok" ? CheckCircle2 : item.level === "error" ? CircleAlert : Info;
          return (
            <li key={`${item.code}-${index}`} data-level={item.level}>
              <Icon size={17} aria-hidden="true" />
              <div>
                <p><span className="diagnostic-level">{item.level === "ok" ? "已检查" : item.level === "error" ? "需修正" : "需确认"}</span>{item.message}</p>
                {item.hint && <small>{item.hint}</small>}
              </div>
            </li>
          );
        })}
      </ul>
      {result.executable && (
        <div className="diagnostic-command"><span>发现的命令文件</span><code>{result.executable}</code></div>
      )}
      <p className="hint">{result.note}</p>
    </section>
  );
}
