## relocation

基于修改时间将图片/音视频按 `YYYY/mm-dd` 结构复制到目标目录，可并行加速。

### 安装
- 开发运行：`cargo run -- --source <src> --dest <dst>`
- 发布二进制：`cargo build --release`，可执行文件位于 `target/release/relo`

### 用法
```bash
relo --source /path/to/source --dest /path/to/dest
# 简写
relo -s /path/to/source -d /path/to/dest
```

主要参数：
- `-s, --source <path>` 源目录（必填）
- `-d, --dest <path>` 目标目录（必填）
- `-j, --jobs <num>` 并行任务数，默认自动按 CPU 选择；机械盘可适当降低
- 日志级别可通过环境变量控制：`RUST_LOG=debug relo ...`

### 行为
- 仅处理常见图片/音视频后缀，忽略 `Thumbs.db`/`.DS_Store`
- 目标已存在同名文件会跳过，不覆盖
- 源、目标同一文件系统时优先尝试硬链接，加速且节省空间，失败再回退到字节复制

### 开发
- 格式化：`cargo fmt`
- 测试：`cargo test`
