# Community Node Critical Safety Architecture

Japanese translation: `docs/safety/community-node-critical-safety_ja.md`

Last updated: 2026-06-25

## Positioning

This document defines the critical safety architecture that kukuri expects before any public community node enables public indexing, discovery, recommendation, or relation outputs for user content. It builds on the current repository implementation and documentation, especially the community-node capability model, operator documentation, P2P-first responsibility boundary, and moderation-event trust semantics.

The current repository implementation provides community-node connectivity and operator-support capabilities such as auth / consent, bootstrap assist, topic rendezvous, iroh relay, and a report endpoint. Community indexing, moderation, and community-local trust are still planned capabilities, not currently shipped capabilities. This document records the safety constraints that must be designed into those future capabilities before they become public content-surfacing paths.

- This is not legal advice. Final operational and legal decisions remain the responsibility of each community node operator, with appropriate expert or regulatory consultation.
- This is not a production provider integration specification. Provider contracts, credentials, and production integrations are outside the current implementation scope described here.
- This document is for provider outreach and architecture alignment before public indexing.

Related implementation and documentation:

- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
- `docs/architecture/moderation-event-trust-semantics.md`
- `docs/runbooks/community-node-operator-docs.md`
- `crates/cn-operator/src/capability.rs`
- `crates/cn-operator/src/manifest.rs`

## 1. Current implementation status

kukuri community nodes are currently in an **early implementation phase**. The current implementation is centered on auth / consent, bootstrap assist, topic rendezvous, iroh relay, and a report endpoint.

- `community_index`, `moderation`, and `community_local_trust` are **Phase B: planned, not provided by this build**. This is reflected by `Availability::Planned` in `crates/cn-operator/src/capability.rs` and by the Phase A / Phase B split in `docs/runbooks/community-node-operator-docs.md`.
- `report_endpoint` is Phase A and is available: operators can receive reports through `POST /v1/report` and inspect them with `cn-cli reports`.
- Content surfacing capabilities such as index, discovery, recommendation, and relation output are not yet implemented.

**Therefore, public indexing must not be enabled until a fail-closed critical safety architecture is implemented.** This document fixes critical safety as an **architecture constraint**, not as a later add-on.

## 2. P2P-first responsibility boundary

A central authority that governs the entire kukuri network does not exist. This is not a policy choice that safety moderation declines to make; it is **structurally impossible** because kukuri is built on a P2P foundation. There is no central chokepoint that owns the network, so no actor — including the kukuri project or any community node — can be positioned as a network-wide governor, even if it wanted to be. The safety architecture is therefore designed to work *within* this constraint because it cannot add a central authority on top of the P2P foundation.

- A community node is a **service provider** for the P2P network, not a home server.
- User identity, profile, and social graph are **node-independent**. They are not owned, frozen, or deleted by any particular community node, because no node holds them as canonical state.
- Safety decisions, including verdicts, moderation events, and risk signals, apply only within the **issuer node's authority scope**. A node may exclude content from the outputs it controls, such as its own index, moderation, cache, discovery, recommendation, or relation outputs.
- Because there is no central authority to invoke, a global moderation authority or a network-wide takedown command is not available to anyone. Even for critical safety, no central actor can force the entire network to follow a moderation decision; each node can only act within its own authority scope.

```text
safety verdict / moderation event = an issuer-node-scoped decision
                                  != network-wide command (no such command can exist on a P2P foundation)
```

## 3. Safety goals

A public community node should have an architecture that can do at least the following:

- Actively exclude CSAM / CSE and other critical safety risks from that node's index, discovery, recommendation, and relation outputs.
- Make active exclusion explainable and auditable through signed moderation events and risk signals.
- Avoid relying on individual or small-scale operators to manually inspect harmful media.
- Preserve the assumption that community nodes do **not** permanently store blob bodies (**no permanent blob storage**).

## 4. Non-goals

- Do not host or distribute a CSAM hash database.
- Do not train an in-house CSAM detection model.
- Do not require operators to manually review CSAM / CSE media.
- Do not describe the kukuri project or default node as a network-wide moderation authority; that role cannot exist on the P2P foundation.
- Do not treat general NSFW moderation as the same route as CSAM / CSE critical safety.

## 5. Component boundary

```text
community node
  - receives posts and references from gossip / docs / nostr / local ingestion
  - does not permanently store blob bodies
  - sends media / external blob references to the moderation server before indexing
  - reflects only `allow` verdicts into the index
  - can store and distribute signed moderation events
  - can reflect risk signals into trustness / relation outputs

moderation server
  - holds provider credentials
  - temporarily fetches blobs when needed, without permanent storage
  - runs scan providers, classifiers, and the policy router
  - returns verdicts
  - creates source data for incident records and moderation events
  - exposes hooks for reporting workflows
```

Providers are modeled by capability, not by a single boolean. Example capabilities include known CSAM hash matching, perceptual hash matching, unknown CSAM / CSE classification, general media moderation, malware / phishing detection, and reporting workflows.

The repository should be able to validate the provider abstraction and readiness behavior with a **mock provider** before any production provider integration is configured.

## 6. Data flow

```text
incoming post / blob reference / metadata
  ↓
community node
  ↓
moderation server
  ↓
policy router
  ├─ known CSAM hash matching
  ├─ unknown CSAM / CSE classifier
  ├─ general media moderation
  ├─ text moderation / grooming suspicion
  ├─ spam / abuse / malware / phishing
  └─ reporting workflow hook
  ↓
safety verdict
  ↓
index / search / discovery / recommendation / relation outputs only when verdict is `allow`
```

## 7. Verdict model and routing

Detection labels and final actions are separated.

- Actions: `allow`, `hold`, `quarantine`, `exclude`
- `csam_confirmed` (known hash match / provider confirmed) is distinct from `csam_suspected` (high classifier score / CSE suspicion).

| Category | Input examples | Routing |
|---|---|---|
| Known CSAM | known hash match / provider confirmed | `exclude` + critical moderation event + risk signal + reporting workflow hook where applicable |
| Suspected unknown CSAM / CSE | high classifier score / CSE suspicion | `hold` or `quarantine` + local-first risk signal + further provider / report workflow route |
| General moderation | nsfw / violence / hate / harassment / spam / malware / phishing | separate non-critical policy route: allow / downrank / hide / exclude |

Suspected unknown CSAM / CSE is not treated as confirmed. It is routed as a suspected critical safety case.

## 8. Fail-closed invariants

The content-surfacing implementation must guarantee the following through database constraints and tests:

- Unscanned media is never indexed.
- Scan failure or provider unavailability never becomes `allow`.
- `hold`, `quarantine`, and `exclude` verdicts are not searchable.
- Critical verdicts never enter discovery or recommendation.
- A public-node profile fails readiness if the mandatory known-CSAM provider is missing.
- Community nodes do not permanently store blob bodies (**no permanent blob storage**).

## 9. Signed moderation events and risk signals

- A signed moderation event identifies the issuer node and the target, such as a post, blob, user, or peer. The issuer node signs the event.
- The event is advisory within the issuer node's authority scope and is **not a network-wide command**.
- A risk signal is not an unsupported label. It carries basis, confidence, severity, and visibility.
- Visibility has three levels: `local`, `subscribed_nodes`, and `public`. **Suspected unknown CSAM / CSE defaults to `local`**. `subscribed_nodes` or wider visibility is considered only for known CSAM hash matches or provider-confirmed results, to avoid spreading false positives as public advisories.

## 10. Reporting, appeal, and operator audit

- Reports are not centralized. They are routed to the authority scope of the node that actually participated in the relevant content surfacing, moderation, cache, or report flow, as represented by `crates/desktop-runtime/src/community_node/report_routing_support.rs` and the community-node manifest model.
- Moderation event and incident records should use event IDs, reference IDs, and basis categories rather than redistributing harmful content as evidence.
- Operators should be able to explain and audit active exclusion without redistributing raw harmful material.

## 11. Readiness before public indexing

Before a public-node profile enables public indexing, it must satisfy at least the following:

- A known-CSAM provider is configured.
- Provider credentials are validated by a readiness check.
- `index_before_scan = false`.
- Scan errors route to hold / fail-closed behavior.
- Signed moderation events are enabled.
- Permanent blob storage is disabled.
- Scan coverage metrics are available.

## 12. Implementation prerequisites

The current repository state does not yet provide public indexing, moderation, or community-local trust as shipped runtime capabilities. Before those capabilities become public content-surfacing paths, the implementation must add and validate at least the following:

- provider abstraction and provider capability modeling
- mock provider coverage for deterministic tests
- policy routing that separates known CSAM, suspected unknown CSAM / CSE, and general moderation
- readiness checks for public-node profiles
- fail-closed indexing constraints
- signed moderation event generation
- risk signal persistence and distribution semantics

Production provider application and production integration should happen after the repository can validate the provider abstraction and readiness checks without relying on production credentials.

## Related references

- Japanese translation: `docs/safety/community-node-critical-safety_ja.md`
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
- `docs/architecture/moderation-event-trust-semantics.md`
- `docs/runbooks/community-node-operator-docs.md`
- `docs/architecture/default-community-node-dependency-reduction.md`
