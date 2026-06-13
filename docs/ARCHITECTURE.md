# Architecture Overview

## Contract Call Flow

```mermaid
flowchart TD
    A([Deployer]) -->|deploy + init| B[init\nadmin, token]
    B --> C[(Contract Initialized)]
    C --> D{Admin}
    D -->|create_stream\nrecipient, amount, start, end| E[create_stream]
    E -->|transfer tokens in| F[(StreamState stored\nper recipient)]
    F --> G{Recipient / Anyone}
    G -->|read-only| H[claimable_amount\nrecipient → i128]
    G -->|require_auth recipient| I[withdraw\nrecipient]
    I -->|transfer unlocked tokens out| J([Recipient Wallet])
```

### Call Flow Description

| Step | Function | Caller | Effect |
|------|----------|--------|--------|
| 1 | `init` | Deployer (once) | Stores admin + token address in instance storage |
| 2 | `create_stream` | Admin only | Transfers tokens into contract; writes `StreamState` to persistent storage |
| 3 | `claimable_amount` | Anyone | Computes `total * elapsed / duration - claimed`; read-only |
| 4 | `withdraw` | Recipient (auth required) | Transfers unlocked tokens out; updates `claimed_amount` |

---

## Storage Layout

```mermaid
flowchart LR
    subgraph Instance Storage
        A[key: Admin\nval: Address]
        B[key: Token\nval: Address]
    end

    subgraph Persistent Storage
        C["key: recipient_1 (Address)\nval: StreamState"]
        D["key: recipient_2 (Address)\nval: StreamState"]
        E["key: recipient_N (Address)\nval: StreamState"]
    end

    A -.->|read on every call| F([Contract Logic])
    B -.->|read on token transfer| F
    C <-->|read / write| F
    D <-->|read / write| F
    E <-->|read / write| F
```

### Storage Description

**Instance Storage** — shared contract-level data, lives as long as the contract instance:

- `Admin` — the `Address` permitted to call `create_stream`.
- `Token` — the SAC token `Address` used for all transfers in/out.

**Persistent Storage** — per-recipient data, keyed by `Address`:

- `StreamState.recipient` — beneficiary address.
- `StreamState.total_amount` — total tokens allocated to this stream.
- `StreamState.claimed_amount` — cumulative tokens already withdrawn.
- `StreamState.start_time` — Unix timestamp (seconds) when vesting begins.
- `StreamState.end_time` — Unix timestamp (seconds) when fully vested.

Each recipient has exactly one independent stream entry. A second `create_stream` call for the same recipient will panic to prevent ambiguity.
