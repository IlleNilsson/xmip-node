# xmip-core-node

The Node: one machine in an Xmip Cluster, its `NodeRole`s and what it
declares. It is the one declaration of what a node is for; anything else that
needs it reads it here rather than carrying a copy (ADR-0044).

`NodeRole` is that declaration (ADR-0056, amendments 2026-10-01): eight
roles — operational, monitoring, receiving, processing, sending, executing,
development, storage. Receiving, processing and sending serve one stage of the
message path each, and executing is their sum, all three in one process, the
low-latency role; storage is Xmip Storage, the doorway every other node calls
for all storage, and serves no stage; `NodeRole::stages` says which stages a role serves, and
`NodeRole::said` says a set of roles one way, the three together as
executing. `NodeRole::declared` is the one parse: words separated by commas
or `+`, each exact and lowercase only (the owner, 2026-09-24), an unknown word
refused by name (ADR-0055); nothing is inferred from a node's name. The
Playground reads every declaration through it, and every other surface calls
it: the runtime's library forwards `NodeRole::WORDS`, `declared` and `stages`
as `xmip_role_words_v1`, `xmip_role_declared_v1` and `xmip_role_stages_v1`
(`xmip_operate.h` section 7), which `Xmip.Surface` and the estate's PowerShell
module call rather than keep a copy.

`Stage` is a stage of the message path — `receive`, `process`, `send` — and
the scope segment it is named by; a node does not declare one. A stage says
whether an operator may pause it (`pausable`) and what a thing configured at
it is called (`location`), and its words are `xmip_stage_words_v1`, all
forwarded the same way.

`Capability` is what a node declares — its roles and whether it may assume
the internet — and the two forms it is said in: the evidence a node
publishes about itself (`evidence`, `from_evidence`) and the entry a run
lists it by (`entry`, `from_entry`: `R1=receiving+sending`). The Playground
declares through it and the surfaces read through it
(`xmip_capability_published_v1`, `xmip_capability_entry_v1`).

A System Process's declaration (ADR-0053) is TOML, its strings quoted by
`xmip-core-library-codec`: every control character is escaped, so any name,
location or path is one line a TOML reader takes back exactly. `Declaration`
writes it — the name, the location and the purpose, and whatever else the
process says of itself (`with`, a bare key and none of the six every
declaration writes), which is how a Playground node says its flags — and
`standing` reads back every declaration in a directory (`Standing`). The
purpose is `test` or `runtime` exactly (`Purpose::declared`); any other word
is REFUSED, as a stage's is. No other language writes or reads a declaration:
the runtime's library forwards both as `xmip_process_declare_v1` and
`xmip_process_declarations_v1` (`xmip_operate.h` section 13), and a declaration
made across that boundary is `handed_over` to its caller, which takes it away
where its process ends.

A Node is not the Xmip Service that runs on it and not a Host Service; those
are `xmip-core-runtime`'s. A Node does not hold cluster membership — the
Cluster does — and it does not decide where work runs.

`doc/architecture/deployment-model.md` section 3 governs the roles a node
combines; `architecture.toml` carries the maturity.
