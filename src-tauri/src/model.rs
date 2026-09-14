use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Stdio,
    Http,
    Sse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
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
pub struct Binding {
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: Config,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub bindings: BTreeMap<String, Binding>,
    #[serde(default)]
    pub native: BTreeMap<String, Value>,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub deleted_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub adapter_id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub id: String,
    pub at: u64,
    pub status: String,
    pub summary: String,
    pub paths: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub version: u32,
    pub revision: u64,
    pub services: Vec<Service>,
    pub targets: Vec<Target>,
    pub history: Vec<History>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    pub id: Option<String>,
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: Config,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub service_id: String,
    pub target_id: String,
    pub target_name: String,
    pub key: String,
    pub action: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub conflict: bool,
    pub message: String,
}

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
                        // Commands and arguments can contain embedded secrets. Reveal only in the explicit editor.
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

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
