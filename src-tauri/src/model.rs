//! 工作区持久化模型、公共配置校验与展示脱敏。
//! Config 描述服务库期望值，Binding 保存确认过的目标原生条目，两者不能混用。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// 跨核心模块使用可直接返回 IPC 的错误文本；调用方不能吞掉写入或恢复错误。
pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// 序列化名称与前端 Transport 一致；HTTP 与 SSE 的原生字段由适配器转换。
pub enum Transport {
    Stdio,
    Http,
    Sse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// 跨客户端的公共配置；stdio 与远程字段互斥，默认空值便于兼容缺失可选字段。
pub struct Config {
    pub transport: Transport,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl Config {
    /// 校验传输字段互斥、地址与文本边界，限制单项配置为 128 KB。
    /// 只验证配置形状，不启动命令、解释环境变量或请求远程地址。
    pub fn validate(&self) -> Result<()> {
        if self.transport == Transport::Stdio {
            if self.command.trim().is_empty() || self.command.contains('\0') {
                return Err("启动命令不能为空或包含空字符".into());
            }
            if !self.url.is_empty() || !self.headers.is_empty() {
                return Err("stdio 配置不能包含 HTTP 地址或请求头".into());
            }
        } else {
            let url = url::Url::parse(&self.url).map_err(|_| "请输入完整的 HTTP(S) 地址")?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                return Err("仅支持 HTTP(S) 服务地址".into());
            }
            if !url.username().is_empty() || url.password().is_some() {
                return Err("请将认证信息放入请求头，不要嵌入 URL".into());
            }
            if !self.command.is_empty()
                || !self.args.is_empty()
                || !self.cwd.is_empty()
                || !self.env.is_empty()
            {
                return Err("远程服务不支持本地命令、工作目录或环境变量；请使用请求头".into());
            }
        }
        if self.args.iter().any(|v| v.contains('\0')) || self.cwd.contains('\0') {
            return Err("参数或路径包含空字符".into());
        }
        if self
            .env
            .iter()
            .any(|(k, v)| k.is_empty() || k.contains(['=', '\0']) || v.contains('\0'))
        {
            return Err("环境变量名称或值无效".into());
        }
        if self.headers.iter().any(|(k, v)| {
            k.is_empty() || k.contains(['\r', '\n', '\0']) || v.contains(['\r', '\n', '\0'])
        }) {
            return Err("请求头名称或值无效".into());
        }
        if serde_json::to_vec(self)
            .map_err(|_| "配置无法序列化")?
            .len()
            > 128 * 1024
        {
            return Err("单个服务配置不能超过 128 KB".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 服务在某个目标的已确认基线，用于识别磁盘中的后续修改。
pub struct Binding {
    /// Some 为完整原生条目；None 表示确认过条目不存在，区别于未建立绑定记录。
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 内部服务身份与期望配置；目标配置键保持稳定，软删除期间仍保留同步信息。
pub struct Service {
    pub id: String,
    /// 目标文件中的服务键；不是内部 ID，也不要求不同来源服务在整个库内全局唯一。
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: Config,
    #[serde(default)]
    /// 用户期望写入的目标 ID；取消分配不会立即修改目标文件。
    pub targets: Vec<String>,
    #[serde(default)]
    /// 按目标 ID 保存基线，取消分配后仍需保留，以生成移除差异。
    pub bindings: BTreeMap<String, Binding>,
    #[serde(default)]
    /// 按适配器 ID 保存来源原生字段；首次分配时作为保留专属字段的转换底稿。
    pub native: BTreeMap<String, Value>,
    #[serde(default)]
    /// 软删除标记；条目留在库中，供预览移除和用户撤销使用。
    pub deleted: bool,
    #[serde(default)]
    /// 软删除前的期望分配列表；撤销时恢复，不参与普通配置编辑。
    pub deleted_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 一个明确配置文件及其适配器；同一适配器可以对应多个自定义目标。
pub struct Target {
    pub id: String,
    pub adapter_id: String,
    pub name: String,
    /// 配置文件的绝对路径；保存目标与实际读写时校验，不计算客户端继承或组织策略。
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 工作区内可展示的事务摘要；完整前后文本存放在同 ID 的独立备份日志中。
pub struct History {
    pub id: String,
    /// Unix 时间戳，单位为秒；前端格式化日期时需要转换为毫秒。
    pub at: u64,
    /// 持久化协议状态，与 Journal 状态及前端 HistoryStatus 保持兼容。
    pub status: String,
    pub summary: String,
    pub paths: Vec<String>,
    /// 服务条目变更数，不是文件数；反向恢复事务可为 0。
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 完整本地服务库；加载时验证格式版本，成功提交时推进修订。
pub struct Workspace {
    /// 持久化格式版本；当前仅接受版本 1，不能与每次保存的 revision 混淆。
    pub version: u32,
    /// 工作区提交序号，作为同步计划失效依据；不是客户端文件的修改时间。
    pub revision: u64,
    /// 旧工作区缺少此字段时不重复引导；新工作区显式设为 false。
    #[serde(default = "onboarding_already_complete")]
    pub onboarding_complete: bool,
    pub services: Vec<Service>,
    pub targets: Vec<Target>,
    pub history: Vec<History>,
}

fn onboarding_already_complete() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 创建／编辑请求只携带用户可编辑字段，避免前端直接改写绑定或事务状态。
pub struct ServiceInput {
    /// None 新建，Some 编辑现有未删除服务；编辑时不能更换配置键。
    pub id: Option<String>,
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: Config,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 一个目标上的一项服务差异；值是否脱敏由调用它的预览接口决定。
pub struct Change {
    pub service_id: String,
    pub target_id: String,
    pub target_name: String,
    pub key: String,
    /// add / update / remove，与前端 ChangeAction 对齐。
    pub action: String,
    /// 本次磁盘原生条目，None 表示该键不存在。
    pub before: Option<Value>,
    /// 服务库期望的原生条目，None 表示计划移除此键。
    pub after: Option<Value>,
    /// 磁盘值偏离已确认基线；即使与期望值相等也需要明确确认。
    pub conflict: bool,
    pub message: String,
}

/// 生成用于展示和模板导出的副本，隐藏已知凭据字段及命令参数，保留必要结构。
/// URL 会清除用户信息、查询和片段；这不是任意文本秘密的通用检测器，不用于存储原文。
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(obj) => Value::Object(
            obj.iter()
                .map(|(k, v)| {
                    let lower = k.to_ascii_lowercase();
                    let sensitive =
                        ["env", "environment", "headers", "http_headers"].contains(&lower.as_str());
                    let val = if sensitive {
                        if let Some(map) = v.as_object() {
                            Value::Object(
                                map.keys()
                                    .map(|key| (key.clone(), Value::String("<已隐藏>".into())))
                                    .collect::<Map<_, _>>(),
                            )
                        } else {
                            Value::String("<已隐藏>".into())
                        }
                    } else if [
                        "token",
                        "secret",
                        "password",
                        "authorization",
                        "api_key",
                        "apikey",
                    ]
                    .iter()
                    .any(|s| lower.contains(s))
                    {
                        Value::String("<已隐藏>".into())
                    } else if ["args", "command"].contains(&lower.as_str()) {
                        // 命令和参数可能直接携带凭据；完整值仅在编辑或显式选择完整显示时呈现。
                        if let Some(values) = v.as_array() {
                            Value::Array(
                                values
                                    .iter()
                                    .map(|_| Value::String("<已隐藏>".into()))
                                    .collect(),
                            )
                        } else {
                            Value::String("<已隐藏>".into())
                        }
                    } else if ["url", "httpurl", "serverurl"].contains(&lower.as_str()) {
                        if let Some(text) = v.as_str() {
                            if let Ok(mut url) = url::Url::parse(text) {
                                url.set_query(None);
                                url.set_fragment(None);
                                let _ = url.set_username("");
                                let _ = url.set_password(None);
                                Value::String(url.to_string())
                            } else {
                                Value::String("<地址表达式>".into())
                            }
                        } else {
                            redact(v)
                        }
                    } else {
                        redact(v)
                    };
                    (k.clone(), val)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        _ => value.clone(),
    }
}

/// 历史记录使用的 Unix 秒数；系统时间早于纪元时回退为 0。
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
/// 生成服务、目标和事务使用的稳定 UUID，不依赖名称或路径。
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
