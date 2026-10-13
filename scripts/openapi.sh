#!/usr/bin/env bash
# Writes the engine's OpenAPI document to web/openapi.json (no server or database needed).
set -euo pipefail
cd "$(dirname "$0")/.."
SQLX_OFFLINE=true cargo run -q -- openapi > web/openapi.json
