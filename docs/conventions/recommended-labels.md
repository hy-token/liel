# Recommended labels

This page gives a small starter vocabulary for `.liel` files used as local AI
memory, project memory, and source-backed notes. It is a convention, not a
schema. Applications may add labels and edge labels freely.

## This page decides

- starter vocabulary (labels, edges, properties)
- minimal naming rules to reduce drift
- minimal identity-key rules for mergeability

## This page does not decide

- provenance workflow details (see `provenance.md`)

## Node labels

| Label | Use for |
|---|---|
| `Actor` | A person, user, agent, or service that takes actions |
| `Project` | A repository, workspace, product, or ongoing effort |
| `Task` | Work to do, planned work, or completed work |
| `Decision` | A choice with consequences or rationale |
| `Note` | General memory that does not need a narrower label |
| `Topic` | A subject, tag-like concept, or area of interest |
| `File` | A local file or repository path |
| `Source` | A document, URL, issue, paper, or other evidence source |
| `Claim` | A statement that may need support, contradiction, or review |
| `ToolResult` | Output from a command, tool call, benchmark, or agent action |
| `Session` | A bounded interaction or work session |
| `Event` | Something that happened at a time |
| `Place` | A location or environment |

Use multiple labels when both are useful, for example `["Source", "File"]` for
a local source file or `["Actor", "Agent"]` for an AI agent.

## Edge labels

| Edge label | Use for |
|---|---|
| `RELATES_TO` | A loose relationship when no sharper edge is useful |
| `MENTIONS` | A note, source, or result mentions another node |
| `HAS_TOPIC` | A node is about a topic |
| `DERIVED_FROM` | A node was created from a source or tool result |
| `SUPPORTS` | Evidence supports a claim or decision |
| `CONTRADICTS` | Evidence conflicts with a claim or decision |
| `DEPENDS_ON` | A task, decision, or file depends on another node |
| `CREATED_BY` | A node was created by an actor or tool |
| `OBSERVED_IN` | A fact or result was observed in a session, source, or event |
| `DECIDED_IN` | A decision belongs to a session, issue, or source |
| `NEXT` | Ordered sequence, such as steps or session flow |

Prefer a specific edge over packing the relationship into text. A graph that
uses explicit edges is easier to inspect and merge than one large note with many
implied relationships.

## Common properties

| Property | Use for |
|---|---|
| `key` | Project-local stable identity |
| `name` | Short human-readable name |
| `title` | Display title for documents, decisions, tasks, or sources |
| `text` | Original short text |
| `summary` | Human or agent-written summary |
| `path` | Project-relative or intentionally absolute file path |
| `url` | Canonical URL |
| `external_id` | ID from another system |
| `system` | Name of the external system for `external_id` |
| `status` | Application-level state such as `open`, `done`, or `archived` |
| `created_at` | RFC 3339 UTC creation time |
| `updated_at` | RFC 3339 UTC update time |

Keep property names boring and explicit. This matters more than choosing the
perfect vocabulary on the first try.

## Minimal naming and normalization rules

- Node labels: `PascalCase` (for example `Task`, `Source`)
- Edge labels: uppercase verb phrases (for example `DEPENDS_ON`, `DERIVED_FROM`)
- Property names: `lower_snake_case`
- Text for comparison: normalize line endings to `\n`
- Paths: prefer project-relative paths with `/` separators

## Minimal identity-key rules

- Prefer `path`, `url`, or `external_id` (optionally with `system`)
- Avoid internal IDs, timestamps, and long free text as identity keys

## Minimal starter set

If you need a small default, start with:

- Node labels: `Project`, `Task`, `Decision`, `Source`, `Note`
- Edge labels: `RELATES_TO`, `DEPENDS_ON`, `DERIVED_FROM`, `SUPPORTS`
- Properties: `key`, `title`, `status`, `path` or `url`, `updated_at`

Expand only when needed.

