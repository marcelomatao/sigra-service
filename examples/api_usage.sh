#!/usr/bin/env bash
# Sigra Service — API smoke test.
# Requires: curl, jq, running sigra-service on localhost:8080 with MongoDB and MinIO.
set -euo pipefail

BASE="http://localhost:8080"
U="test-user-uuid-1234"

echo "=== Health ===" && curl -s "$BASE/health" | jq .

echo -e "\n=== Upload Document ==="
DOC=$(curl -s -X POST "$BASE/api/documents" -H "X-User-UUID: $U" -F "file=@README.md")
echo "$DOC" | jq . && DOC_ID=$(echo "$DOC" | jq -r '.id')

echo -e "\n=== Get Document ==="
curl -s "$BASE/api/documents/$DOC_ID" -H "X-User-UUID: $U" | jq .

echo -e "\n=== Download URL ==="
curl -s "$BASE/api/documents/$DOC_ID/download" -H "X-User-UUID: $U" | jq .

echo -e "\n=== Create Envelope ==="
ENV=$(curl -s -X POST "$BASE/api/envelopes" -H "X-User-UUID: $U" \
  -H "Content-Type: application/json" \
  -d "{\"document_id\":\"$DOC_ID\",\"title\":\"Test\"}")
echo "$ENV" | jq . && ENV_ID=$(echo "$ENV" | jq -r '.id')

echo -e "\n=== Add Signer ==="
SIG=$(curl -s -X POST "$BASE/api/envelopes/$ENV_ID/signers" -H "X-User-UUID: $U" \
  -H "Content-Type: application/json" -d '{"name":"Alice","email":"alice@example.com"}')
echo "$SIG" | jq . && SIG_ID=$(echo "$SIG" | jq -r '.id')

echo -e "\n=== Send ==="
curl -s -X POST "$BASE/api/envelopes/$ENV_ID/send" \
  -H "X-User-UUID: $U" -H "Content-Type: application/json" -d '{}' | jq .

echo -e "\n=== Sign ==="
curl -s -X POST "$BASE/api/envelopes/$ENV_ID/sign" \
  -H "Content-Type: application/json" \
  -d "{\"signer_id\":\"$SIG_ID\",\"signature_data\":\"email_confirmed\"}" | jq .

echo -e "\n=== Envelope ==="
curl -s "$BASE/api/envelopes/$ENV_ID" -H "X-User-UUID: $U" | jq .

HASH=$(echo "$DOC" | jq -r '.hash')
echo -e "\n=== Verify by hash ==="
curl -s "$BASE/api/verify/hash/$HASH" | jq .

echo -e "\n=== Trigger anchor ===" && curl -s -X POST "$BASE/admin/anchor" | jq .

echo -e "\nDone."
