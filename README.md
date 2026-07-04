# pinocchio-fundraiser

A [pinocchio](https://github.com/anza-xyz/pinocchio) port of [ASCorreia/anchor-fundraiser](https://github.com/ASCorreia/anchor-fundraiser).

An SPL-token fundraiser: a maker sets a target and duration, contributors fund a vault
(capped at 10% of the target each), and after the campaign the maker claims the funds if the
target was met or contributors refund if it wasn't.

## Instructions

| # | Name | Signer | Description |
|---|------|--------|-------------|
| 0 | `Initialize` | maker | Creates the fundraiser PDA and its vault ATA. |
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

## Build & test

```sh
cargo build-sbf
cargo test
```
