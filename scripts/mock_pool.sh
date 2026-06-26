#!/usr/bin/env bash
# mock_pool.sh — Create a mock milestone pool on the Stellar testnet for local dev/testing.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Load .env if present
[ -f "$PROJECT_ROOT/.env" ] && source "$PROJECT_ROOT/.env"

NETWORK="${STELLAR_NETWORK:-testnet}"
IDENTITY="${STELLAR_IDENTITY:-default}"
CONTRACT_ID="${CONTRACT_ID:-}"
ASSET="${MOCK_ASSET:-native}"
TOTAL_FUNDS="${TOTAL_FUNDS:-1000}"
GUARD_CONTRACT="${GUARD_CONTRACT:-}"

usage() {
    echo "Usage: $0 [--contract <id>] [--guard <id>] [--asset <address>] [--funds <amount>] [--network <name>] [--identity <name>]"
    exit 1
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --contract)  CONTRACT_ID="$2";   shift 2 ;;
        --guard)     GUARD_CONTRACT="$2"; shift 2 ;;
        --asset)     ASSET="$2";         shift 2 ;;
        --funds)     TOTAL_FUNDS="$2";   shift 2 ;;
        --network)   NETWORK="$2";       shift 2 ;;
        --identity)  IDENTITY="$2";      shift 2 ;;
        *)           usage ;;
    esac
done

if [ -z "$CONTRACT_ID" ] || [ -z "$GUARD_CONTRACT" ]; then
    echo "Error: --contract and --guard are required (or set CONTRACT_ID / GUARD_CONTRACT in .env)."
    usage
fi

echo "==> Creating mock milestone pool..."
echo "    Network:        $NETWORK"
echo "    Contract:       $CONTRACT_ID"
echo "    Guard:          $GUARD_CONTRACT"
echo "    Asset:          $ASSET"
echo "    Total funds:    $TOTAL_FUNDS"

stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$IDENTITY" \
    --network "$NETWORK" \
    -- \
    create_milestone_pool \
    --guard_contract "$GUARD_CONTRACT" \
    --asset "$ASSET" \
    --total_funds "$TOTAL_FUNDS"

echo "==> Mock milestone pool created successfully."
