# prompt

Formats structured context into a single prompt string for AI models.

## Purpose

- **Single source of truth** for prompt layout: system message, user preferences, conversation (recent), and semantic reference.
- **No dependency on memory**: accepts optional strings and message slices; used by the `memory` crate but usable from any context source.
- **Section titles**: `Conversation (recent):` and `Relevant reference (semantic):` so the model can tell main dialogue from retrieved context.

## API

- `format_for_model(include_system, system_message, user_preferences, recent_messages, semantic_messages) -> String`
- Constants: `SECTION_RECENT`, `SECTION_SEMANTIC`

## Usage

Used by `memory::Context::format_for_model()`. You can also call `prompt::format_for_model()` directly with your own system message, preferences, and message lists.

## External interactions

Output is intended for LLM APIs (OpenAI, Anthropic, etc.).
