# API Endpoints

## 智谱AI (Zhipu AI)

- **地址**: `POST https://open.bigmodel.cn/api/paas/v4/chat/completions`
- **格式**: 兼容 **OpenAI**（非 Anthropic）。请求体示例：`model`、`messages`、`temperature`、`max_tokens`、`stream`；请求头：`Authorization: Bearer YOUR_API_KEY`、`Content-Type: application/json`。
- **常用模型**: glm-4、glm-4-flash、glm-4-air、glm-3-turbo。
- **环境变量**: `ZHIPU_API_KEY`、`ZHIPU_API_URL`（可选，默认上述地址）。

## Anthropic (Claude)

- 公司: Anthropic PBC；API 格式与智谱/OpenAI 不同，本项目使用 OpenAI 兼容端点（智谱等）。
