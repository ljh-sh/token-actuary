# token-actuary（Token 精算师）

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> 隐私优先的大模型输入防火墙与算力成本精算师。  
> 本地化 Token 计数、脱敏、截断与越狱 Token 检测 —— 无网络、无数据泄露。

`token-actuary`（二进制名：`token-actuary`）是一个小巧、自包含的 Rust 工具，在提示词离开机器之前完成审计。它加载 Hugging Face 的 `tokenizer.json`，统计 Token 数、脱敏敏感信息、在 Token 边界安全截断，并标记可能提示注入或越狱的控制 Token。

## 为什么做 token-actuary？

现有的云端 Token 网关必须先把提示词发送到第三方。`token-actuary` 全程本地运行：

- **零网络**: tokenizer 从本地文件加载，或由前端一次性拉取；提示词文本不会离开进程。
- **Token 级安全**: 截断发生在 `Vec<u32>` 上，而不是对原始字符串切片，因此多字节字符和代码块不会断裂。
- **Agent 友好**: 纯 stdout，支持 JSON/TSV，无 TUI、无进度条。
- **WASM 就绪**: 同一核心可编译为 WebAssembly，用于浏览器端审计沙盒。

## 安装

### Cargo

```bash
cargo install token-actuary
```

### 源码构建

```bash
git clone https://github.com/ljh-sh/token-actuary
cd token-actuary
cargo build --release   # 二进制在 target/release/token-actuary
```

## 用法

所有命令都需要 `tokenizer.json`。可通过 `--tokenizer` 传入或设置环境变量 `TOKENIZER_JSON`：

```bash
export TOKENIZER_JSON=/path/to/tokenizer.json
```

### TL;DR 速查

```bash
# 统计 Token（默认使用内置 gpt-4o）
echo "hello world" | token-actuary count

# 用开源 tokenizer 统计中文
echo "你好世界" | token-actuary count --tokenizer qwen2_5.tokenizer.json

# 审计：脱敏敏感信息、截断到预算、输出 JSON 给 Agent
cat prompt.txt | token-actuary audit \
  --redact password,secret,token \
  --replace [REDACTED],[REDACTED],[REDACTED] \
  --max-tokens 4096 --format json

# 对比多个模型的 Token 数
echo "hello world" | token-actuary compare

# 下载推荐的开源 tokenizer
token-actuary download --recommend

# 大陆网络优先走镜像下载
TA_CHINA=1 token-actuary download --recommend

# 编码 / 解码（默认分隔符为 `,`）
echo "hello world" | token-actuary encode
token-actuary decode 24912,2375

# 回环：编码后再解码
echo "hello world" | token-actuary encode | token-actuary decode

# 使用自定义分隔符
echo "hello world" | token-actuary encode -s " | " | token-actuary decode -s " | "

# 打印每个 token 的偏移热图
echo "hello world" | token-actuary heatmap
```

### 统计 Token

```bash
echo "hello world" | token-actuary count
# 2
```

### 审计（脱敏 + 截断 + 检测）

```bash
echo "my secret password is here" | token-actuary audit --redact secret,password --replace [REDACTED],[SECRET] --max-tokens 10
```

输出：

```text
tokens_before: 6
tokens_after:  6
truncated:     false
redactions:    2
jailbreak:     0
---
my [REDACTED] [SECRET] is here
```

JSON 模式便于 Agent 解析：

```bash
cat prompt.txt | token-actuary audit --max-tokens 2048 --format json
```

### 编码 / 解码

```bash
echo "hello world" | token-actuary encode
# 24912,2375,198

token-actuary decode 24912,2375,198
# hello world
```

### 热力图

```bash
echo "hello world" | token-actuary heatmap
```

## 库用法

```rust
use token_actuary::{Actuary, AuditOptions};

let actuary = Actuary::from_file("tokenizer.json")?
    .with_redactions(&["secret", "password"], &["[REDACTED]", "[SECRET_ID_1]"])?
    .with_control_token_prefixes(&["<|im_start|>", "<|endoftext|>"]);

let report = actuary.audit("my secret is safe", &AuditOptions::default())?;
println!("{} tokens", report.tokens_after);
```

## WebAssembly

使用 `wasm-pack` 构建：

```bash
wasm-pack build --target web --features wasm
```

`WasmActuary` 类向 JavaScript 暴露 `count`、`encode`、`decode` 和 `audit`。

## 模型支持

`token-actuary` 使用 Hugging Face `tokenizers` Rust 库。支持所有附带 `tokenizer.json` 的模型（Qwen2.5、Llama3、DeepSeek 等）。大型 tokenizer 文件可进行 Brotli/Gzip 压缩用于 Web 分发，在浏览器解压后实例化。

GGUF 支持尚未内置；我们正在评估是否能用现有 `tokenizers` 的 BPE/WordPiece/Unigram 实现覆盖 GGUF 词表加载，而不新增专用依赖。

## 安全

参见 [SECURITY.md](SECURITY.md)。漏洞请邮件 [lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com)，勿开公开 issue。

## 许可证

Apache 2.0 —— 参见 [LICENSE](LICENSE)。
