# 市场风格样本热更新

市场风格样本由 `docs/market/stable.json` 指向一个不可变的版本定义。定义控制风格名称、展示文案、东方财富板块、采样权重、样本门槛、刷新周期和完整离线兜底成分；评分算法仍由客户端版本控制。

## 发布新定义

1. 复制 `docs/market/definitions/2026.07-v5.json`，以新的 `definition_version` 命名。
2. 修改板块和兜底成分，并设置不早于计划启用日的 `effective_from`。
3. 将 `docs/market/stable.json` 的版本和文件名指向新定义。
4. 提交到 `main`。`Publish market definition` 工作流会运行 Rust 校验、使用 Tauri 更新密钥签名，并把定义、签名和最终指针依次上传到 `market-data` Release。

客户端每天检查一次远程定义。新定义先进入待生效缓存，只在 09:25–09:29 的竞价最终阶段切换；在线成分获取失败时使用同一份签名定义中的 `fallback_groups`。签名、格式、版本、板块代码或覆盖门槛任一校验失败，客户端都会继续使用最后有效版本。

## 可热更新边界

- `styles`: 风格名称、说明和子板块。
- `sample_weight`: 子板块抽样与指数确认权重，范围为 1–10。
- `limits`: 总样本和单风格样本门槛。
- `member_refresh_days`: 在线成分刷新周期，范围为 1–30 天。
- `fallback_groups`: 完整离线兜底样本。
- `effective_from`、`min_app_version` 和定义版本。

评分公式、质量判定算法、接口解析、签名公钥和页面结构不能通过定义热更新。

## 回滚

将 `stable.json` 指回已经发布且仍保留在 `market-data` Release 中的旧定义，再触发工作流。客户端会把旧版本作为新的待生效定义，并在下一个竞价最终阶段切回；本地同时保留上一份有效定义。
