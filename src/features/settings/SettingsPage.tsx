import { ArrowLeft, Check, Monitor, Moon, Palette, Sun } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { useTheme, type Theme, type Palette as ColorPalette } from "@/theme";
import { ErrorBox } from "@/components";

const themes = [
  { value: "light", label: "浅色", detail: "明亮、清晰，适合日间工作", icon: Sun },
  { value: "dark", label: "深色", detail: "柔和炭灰，减少暗光下的眩光", icon: Moon },
  { value: "system", label: "跟随系统", detail: "随系统外观自动切换", icon: Monitor },
] as const;

export function SettingsPage({ onBack, version = "—" }: { onBack: () => void; version?: string }) {
  const { theme, resolvedTheme, setTheme, palette, setPalette, appearanceError } = useTheme();
  return (
    <section className="preferences" aria-label="偏好设置">
      <div className="preferences-content">
        <Button variant="ghost" size="sm" className="back-link" onClick={onBack}>
          <ArrowLeft />返回服务库
        </Button>
        <header className="preferences-heading">
          <div className="eyebrow">MAKE IT YOURS</div>
          <h1>偏好设置</h1>
          <p>让 MCP Deck 更贴合你的工作习惯。</p>
        </header>
        <Card className="appearance-card">
          <CardHeader>
            <div className="appearance-title"><Palette size={19} /><CardTitle>外观</CardTitle></div>
            <CardDescription>选择你喜欢的界面主题，整个工作区会即时更新。</CardDescription>
          </CardHeader>
          <CardContent>
            <ErrorBox text={appearanceError} />
            <RadioGroup className="theme-options" aria-label="界面主题" value={theme} onValueChange={(value) => setTheme(value as Theme)}>
              {themes.map(({ value, label, detail, icon: Icon }) => (
                <label key={value} className={`theme-option ${theme === value ? "selected" : ""}`}>
                  <div className={`theme-preview preview-${value}`} aria-hidden="true">
                    <div className="mini-sidebar"><i /><i /><i /><i /></div>
                    <div className="mini-list"><i /><i /><i /></div>
                    <div className="mini-detail"><b /><i /><i /><span /></div>
                  </div>
                  <div className="theme-option-title"><Icon size={16} /><strong>{label}</strong><RadioGroupItem value={value} aria-label={label} /></div>
                  <span className="theme-option-description">{detail}</span>
                </label>
              ))}
            </RadioGroup>
            <Separator className="my-6" />
            <div className="palette-heading"><h3>配色</h3><p>保留经典绿，或使用简洁的石墨色。</p></div>
            <RadioGroup className="palette-options" aria-label="配色" value={palette} onValueChange={(value) => setPalette(value as ColorPalette)}>
              <label className={`palette-option ${palette === "graphite" ? "selected" : ""}`}>
                <span className="palette-swatches graphite-swatches" aria-hidden="true"><i /><i /><i /></span>
                <span><strong>石墨</strong><small>清晰、克制的中性色</small></span>
                <RadioGroupItem value="graphite" aria-label="石墨" />
              </label>
              <label className={`palette-option ${palette === "green" ? "selected" : ""}`}>
                <span className="palette-swatches green-swatches" aria-hidden="true"><i /><i /><i /></span>
                <span><strong>经典绿</strong><small>熟悉的鼠尾草绿，柔和自然</small></span>
                <RadioGroupItem value="green" aria-label="经典绿" />
              </label>
            </RadioGroup>
            <Separator className="my-6" />
            <div className="theme-status" role="status">
              <span><Check size={15} />当前为{resolvedTheme === "dark" ? "深色" : "浅色"}主题{theme === "system" ? " · 跟随系统" : ""}</span>
              <span>自动记住你的选择</span>
            </div>
          </CardContent>
        </Card>
        <div className="preferences-about"><span>MCP Deck <Badge variant="outline">{version}</Badge></span><span>一个服务库，连接你的 AI 工具。</span></div>
      </div>
    </section>
  );
}
