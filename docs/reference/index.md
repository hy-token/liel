# Behavior and specifications

This section describes **how liel behaves today**: feature coverage, reliability, and the byte-level file format. For the rationale behind the scope, see [Design](../design/index.md).

## Document scope

| Document | Concern | Read when you need to |
|---|---|---|
| [Feature list](features.md) | Public API and feature coverage | Check what `liel` provides |
| [Reliability and failure model](reliability.md) | Commits, recovery, failure modes, operational assumptions | Use `liel` as durable state |
| [Format spec](format-spec.md) | Byte-level `.liel` file layout | Build compatibility tooling or connectors |

`format-spec.ja.md` and `format-spec.md` cover the same concern. The Japanese `format-spec.ja.md` is the maintainer source of truth; `format-spec.md` is the English public-site version. They should not diverge in scope.

---

| Document | Content |
|---|---|
| [Feature list](features.md) | Quick reference of provided functionality |
| [Reliability and failure model](reliability.md) | What committed data means, how crash recovery works, and which failure modes are out of scope |
| [Format spec](format-spec.md) | Canonical `.liel` byte layout |
