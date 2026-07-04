# pinocchio-fundraiser

A [pinocchio](https://github.com/anza-xyz/pinocchio) port of [ASCorreia/anchor-fundraiser](https://github.com/ASCorreia/anchor-fundraiser).

An SPL-token fundraiser: a maker sets a target and duration, contributors fund a vault
(capped at 10% of the target each), and after the campaign the maker claims the funds if the
target was met or contributors refund if it wasn't.

## Instructions

| # | Name | Signer | Description |
|---|------|--------|-------------|
| 0 | `Initialize` | maker | Creates the fundraiser PDA; validates the client-created vault ATA. |
| 1 | `Contribute` | contributor | Transfers tokens to the vault while the campaign is live. |
| 2 | `CheckContributions` | maker | If the vault reached the target, sends it to the maker and closes the fundraiser. |
| 3 | `Refund` | contributor | After the deadline, if the target was missed, returns the contribution and closes the contributor account. |

## PDAs

- `fundraiser` — `["fundraiser", maker]`
- `contributor` — `["contributor", fundraiser, contributor]`
- `vault` — associated token account of `fundraiser`

## Note

The contribute/refund time-window checks are the reverse of the original Anchor program, whose
logic is inverted (contributions only pass after the deadline, refunds only before). Here
contributions are allowed only before the deadline and refunds only after.

## Optimizations

- Client passes the fundraiser bump; verified with `derive_address` (no `find_program_address` at init).
- ATAs (vault, maker) are created by the client; the program only validates them — no ATA-creation CPI.
- Vault is pinned by its address, stored in the fundraiser account.
- Redundant mint reads and accounts dropped from contribute / check / refund.
- Unused deps removed (`pinocchio-log`, `pinocchio-associated-token-account`).

Contributor PDA derivation stays canonical (`find_program_address`) so the 10% per-wallet cap
can't be bypassed with non-canonical bumps.

Compute units (litesvm):

| Instruction | Before | After |
|---|---|---|
| initialize | 19506 | 1896 |
| contribute | 4575 | 4507 |
| check_contributions | 17755 | 1496 |
| refund | 4879 | 2048 |

## Build & test

```sh
cargo build-sbf
cargo test
```
