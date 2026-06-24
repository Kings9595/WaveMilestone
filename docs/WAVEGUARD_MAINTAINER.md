# WaveGuard Maintainer Identity Requirements

This document describes the identity verification requirements for becoming a WaveGuard maintainer in the WaveMilestone ecosystem.

## Overview

WaveGuard is the identity and access registry that WaveMilestone relies on to verify maintainer authority. Every call to `create_milestone_pool`, `release_issue_bounty`, and `clawback_expired_funds` requires the caller to be a registered maintainer in an associated WaveGuard contract instance.

## Maintainer Eligibility

To become a WaveGuard maintainer, the following criteria must be met:

### 1. Stellar Account

- A funded Stellar account with a valid Ed25519 keypair.
- The account must have sufficient XLM balance to cover transaction fees and minimum account reserve.
- For testnet: fund via Friendbot (`soroban config identity fund <IDENTITY>`).
- For mainnet: acquire XLM from an exchange or existing wallet.

### 2. Identity Verification

- **Human verification**: The maintainer must be a known contributor to the repository or project. Automated or anonymous registrations are not permitted.
- **GitHub association**: The maintainer's Stellar public key must be associated with a GitHub account that has write access to the repository.
- **Off-chain agreement**: Maintainers must agree to the project's code of conduct and terms of participation.

### 3. Technical Requirements

- Ability to sign Soroban contract invocations using the maintainer's Stellar secret key.
- Familiarity with the Soroban CLI and Stellar network operations.
- Understanding of the WaveMilestone contract lifecycle (pool creation, bounty release, clawback).

## Registration Process

1. **Generate a Stellar keypair**:
   ```bash
   soroban config identity generate MAINTAINER_IDENTITY
   ```

2. **Fund the account**:
   ```bash
   # Testnet
   soroban config identity fund MAINTAINER_IDENTITY
   # Mainnet — acquire XLM externally and fund the account
   ```

3. **Register in WaveGuard**:
   The repository administrator deploys a WaveGuard contract instance and registers the maintainer's address:
   ```bash
   soroban contract invoke \
     --id <WAVEGUARD_ID> \
     --source <ADMIN_KEY> \
     --network testnet \
     -- \
     register_maintainer \
     --address <MAINTAINER_PUBLIC_KEY>
   ```

4. **Verify registration**:
   ```bash
   soroban contract invoke \
     --id <WAVEGUARD_ID> \
     --source <MAINTAINER_KEY> \
     --network testnet \
     -- \
     is_maintainer \
     --address <MAINTAINER_PUBLIC_KEY>
   ```

## Maintainer Responsibilities

- **Authorization**: Only call `release_issue_bounty` for legitimate, completed issues.
- **Security**: Protect the maintainer's Stellar secret key. Never share it or commit it to version control.
- **Timeliness**: Monitor milestone expiry dates and claw back unclaimed funds promptly if needed.
- **Compliance**: Adhere to the project's governance and any applicable legal or regulatory requirements.

## Multi-Signature Maintainers

For high-value mainnet deployments, it is strongly recommended to use a multisig setup:

1. Create a Stellar multisig account with an `n-of-m` signing threshold (e.g., 2-of-3).
2. Register the multisig account address in WaveGuard as the maintainer.
3. Each `release_issue_bounty` call requires signatures from the threshold number of authorized signers.

This prevents a single compromised key from authorizing unauthorized payouts.

## Revocation

Maintainer access can be revoked by the WaveGuard administrator:

```bash
soroban contract invoke \
  --id <WAVEGUARD_ID> \
  --source <ADMIN_KEY> \
  --network testnet \
  -- \
  remove_maintainer \
  --address <MAINTAINER_PUBLIC_KEY>
```

Once revoked, the address can no longer call protected WaveMilestone methods.

## References

- [WaveGuard Repository](https://github.com/anomalyco/waveguard)
- [Stellar Account Creation](https://developers.stellar.org/docs/guides/get-started/create-account)
- [Soroban CLI Setup](https://soroban.stellar.org/docs/getting-started/setup)
- [Multisig on Stellar](https://developers.stellar.org/docs/glossary/multisig)
