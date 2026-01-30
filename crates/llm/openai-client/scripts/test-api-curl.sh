#!/usr/bin/env bash
# Test OpenAI-compatible API with curl (e.g. Zhipu open.bigmodel.cn)
# Usage: set env vars then run, e.g.:
#   export OPENAI_API_KEY=your_key
#   export OPENAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4
#   ./scripts/test-api-curl.sh
# Or one-liner: OPENAI_API_KEY=xxx OPENAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4 ./scripts/test-api-curl.sh

set -e

BASE_URL="${OPENAI_BASE_URL:-https://api.openai.com/v1}"
# Zhipu uses glm-4-flash; official OpenAI uses gpt-3.5-turbo
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
    "messages": [{"role": "user", "content": "Hello, please introduce yourself in one sentence."}],
    "max_tokens": 100
  }' | jq .
