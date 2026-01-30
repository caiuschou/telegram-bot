#!/usr/bin/env bash
# 使用 curl 测试 OpenAI 兼容 API（如智谱 open.bigmodel.cn）
# 用法：先设置环境变量再执行，例如：
#   export OPENAI_API_KEY=your_key
#   export OPENAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4
#   ./scripts/test-api-curl.sh
# 或一行：OPENAI_API_KEY=xxx OPENAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4 ./scripts/test-api-curl.sh

set -e

BASE_URL="${OPENAI_BASE_URL:-https://api.openai.com/v1}"
# 智谱用 glm-4-flash；官方 OpenAI 用 gpt-3.5-turbo
MODEL="${MODEL:-glm-4-flash}"
if [[ "$BASE_URL" == *"bigmodel"* ]]; then
  MODEL="${MODEL:-glm-4-flash}"
fi

if [[ -z "${OPENAI_API_KEY:-}" ]]; then
  echo "Error: OPENAI_API_KEY is not set."
  exit 1
fi

echo "Testing: $BASE_URL/chat/completions (model=$MODEL)"
echo "---"

curl -sS -X POST "$BASE_URL/chat/completions" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "'"$MODEL"'",
    "messages": [{"role": "user", "content": "你好，请用一句话介绍你自己。"}],
    "max_tokens": 100
  }' | jq .
