# Behavior and specifications

This section describes **how liel behaves today**: feature coverage, reliability, and the byte-level file format. For the rationale behind the scope, see [Design](../design/index.md).

## Document scope

| Document | Concern | Read when you need to |
|---|---|---|
| [Feature list](features.md) | Public API and feature coverage | Check what `liel` provides |
| [Reliability and failure model](reliability.md) | Commits, recovery, failure modes, operational assumptions | Use `liel` as durable state |
| [Benchmarks and file size notes](benchmarks.md) | Local benchmark script and practical `.liel` size estimates | Interpret benchmark output or estimate memory-file size |
| [Format spec](format-spec.md) | Byte-level `.liel` file layout | Build compatibility tooling or connectors |

`format-spec.ja.md` and `format-spec.md` cover the same concern. The maintainer source of truth is `format-spec.ja.md`; `format-spec.md` is the English public reference. They should not diverge in scope.

---

| Document | Content |
|---|---|
| [Feature list](features.md) | Quick reference of provided functionality |
| [Reliability and failure model](reliability.md) | What committed data means, how crash recovery works, and which failure modes are out of scope |
| [Benchmarks and file size notes](benchmarks.md) | How to read the benchmark script and practical `.liel` size estimates |
| [Format spec](format-spec.md) | Canonical `.liel` byte layout |
