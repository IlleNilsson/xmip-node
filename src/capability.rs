//! What a node declares, and the two forms it says it in (ADR-0056).
//!
//! ADR-0056: *a node declares its capabilities, and work is placed on a node
//! whose capabilities satisfy what the work requires.* A name is not a
//! criterion. Two things are declared here:
//!
//!   - **Roles** — the node's [`NodeRole`]s, the one declaration of what it
//!     is for. Receiving, processing and sending each serve one stage of the
//!     message path, executing serves all three in one process, and the
//!     stages a node serves are read from its roles and nowhere else
//!     (amendment 2026-10-01: the stage words a node used to declare were the
//!     same thing under a second name, and are gone). A node that declares no
//!     stage-serving role runs whole tests itself.
//!   - **Online capability** — whether a route off this machine may be
//!     assumed (ADR-0045).
//!
//! Authentication and runtime capability are not modelled, and the evidence a
//! node publishes says so rather than staying silent.
//!
//! A declaration is said in two forms, each written and read here and
//! nowhere else (open problem 25): the **evidence** a node publishes about
//! itself — `declares receiving,sending; online; …` — and the **entry** a run
//! lists it by — `<name>=receiving+sending`, or the bare name for a node
//! that declares no role. The runtime's library forwards both readings to the
//! surfaces (`xmip_capability_published_v1`, `xmip_capability_entry_v1`,
//! `xmip_operate.h` section 7), so `Xmip.Surface` keeps no parse of its own.

use crate::{NodeRole, Stage};

/// What a node that declares no role publishes in place of the words.
const NO_ROLE: &str = "no role";

/// What the evidence of a declaration starts with.
const DECLARES: &str = "declares ";

/// What one node declared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capability {
    /// Online capability: a route off this machine may be assumed.
    online: bool,
    /// The node's roles, as [`NodeRole::said`] says them.
    roles: Vec<NodeRole>,
}

impl Capability {
    /// A node that declares nothing: no role, offline.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            online: false,
            roles: Vec::new(),
        }
    }

    /// The node declaring these roles, said as [`NodeRole::said`] says them.
    #[must_use]
    pub fn of(roles: &[NodeRole]) -> Self {
        Self {
            online: false,
            roles: NodeRole::said(roles),
        }
    }

    /// The node serving these stages: the role serving each, said as one
    /// declaration — all three are executing.
    #[must_use]
    pub fn serving(stages: &[Stage]) -> Self {
        let roles: Vec<NodeRole> = stages.iter().copied().map(NodeRole::serving).collect();
        Self::of(&roles)
    }

    /// The declaration a text names, read by [`NodeRole::declared`]: role
    /// words separated by commas or by `+`, lowercase exactly. Empty
    /// declares nothing.
    ///
    /// # Errors
    ///
    /// When a word is no role: REFUSED, naming the word and the words there
    /// are (ADR-0055).
    pub fn parse(raw: &str) -> Result<Self, String> {
        NodeRole::declared(raw).map(|roles| Self::of(&roles))
    }

    /// The same declaration with its online capability said (ADR-0045).
    #[must_use]
    pub fn with_online(mut self, online: bool) -> Self {
        self.online = online;
        self
    }

    /// Whether a route off this machine may be assumed.
    #[must_use]
    pub const fn is_online(&self) -> bool {
        self.online
    }

    /// The node's roles, in declaration order.
    #[must_use]
    pub fn roles(&self) -> &[NodeRole] {
        &self.roles
    }

    /// The stages of the message path the node's roles serve, in path order.
    #[must_use]
    pub fn stages(&self) -> Vec<Stage> {
        Stage::ALL
            .into_iter()
            .filter(|stage| self.can(*stage))
            .collect()
    }

    /// Whether one of the node's roles serves `stage`.
    #[must_use]
    pub fn can(&self, stage: Stage) -> bool {
        self.roles.iter().any(|role| role.stages().contains(&stage))
    }

    /// Whether the node carries the whole Journey in its one process: the
    /// executing role (ADR-0018 clause 10a).
    #[must_use]
    pub fn executes(&self) -> bool {
        self.roles.contains(&NodeRole::Executing)
    }

    /// Whether no role of the node serves a stage of the message path — it
    /// runs whole tests itself, as every node did before capabilities.
    #[must_use]
    pub fn declares_no_stage(&self) -> bool {
        Stage::ALL.into_iter().all(|stage| !self.can(stage))
    }

    /// The roles as a declaration: `receiving,processing`, or the empty
    /// string when none is declared.
    #[must_use]
    pub fn words(&self) -> String {
        self.joined(",")
    }

    /// The word a health record carries for the online capability.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        if self.online { "online" } else { "offline" }
    }

    /// What the node's own capability record says, so a surface reads a
    /// node's roles from the snapshot and never from its name.
    #[must_use]
    pub fn evidence(&self) -> String {
        let declared = if self.roles.is_empty() {
            NO_ROLE.to_string()
        } else {
            self.words()
        };
        format!(
            "{DECLARES}{declared}; {}; authentication and runtime capability \
             are not declared in this record",
            self.word()
        )
    }

    /// The declaration an evidence line says, read through the same parse as
    /// a declaration. A line that is no declaration at all declares nothing,
    /// offline unless it says `; online;`.
    ///
    /// # Errors
    ///
    /// When the declaration names a word that is no role: REFUSED, as
    /// [`NodeRole::declared`] says it — never read as the words that were
    /// known.
    pub fn from_evidence(evidence: &str) -> Result<Self, String> {
        let said = evidence
            .strip_prefix(DECLARES)
            .and_then(|rest| rest.split(';').next())
            .unwrap_or_default();
        let roles = if said == NO_ROLE {
            Vec::new()
        } else {
            NodeRole::declared(said)?
        };
        Ok(Self::of(&roles).with_online(evidence.contains("; online;")))
    }

    /// The node as a run lists it: `<name>=receiving+sending`, or the bare
    /// name when it declares no role. It says nothing of the online
    /// capability, which a run lists apart.
    #[must_use]
    pub fn entry(&self, name: &str) -> String {
        if self.roles.is_empty() {
            name.to_string()
        } else {
            format!("{name}={}", self.joined("+"))
        }
    }

    /// An entry read back: the name before the first `=`, trimmed, and the
    /// declaration the rest says — or, when it names a word that is no role,
    /// the refusal [`Capability::parse`] gives. Whatever the node is called
    /// is read as a name and nothing else, refused or not.
    pub fn from_entry(entry: &str) -> (&str, Result<Self, String>) {
        let (name, declared) = entry.split_once('=').unwrap_or((entry, ""));
        (name.trim(), Self::parse(declared))
    }

    fn joined(&self, separator: &str) -> String {
        self.roles
            .iter()
            .map(|role| role.name())
            .collect::<Vec<&str>>()
            .join(separator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_declares_roles_by_name_and_an_unknown_word_is_refused() {
        let one = Capability::parse("receiving").expect("receiving is a role");
        assert_eq!(one.roles(), [NodeRole::Receiving]);
        assert_eq!(one.stages(), [Stage::Receive]);
        assert!(one.can(Stage::Receive) && !one.can(Stage::Send));
        assert!(!one.declares_no_stage() && !one.executes());

        let two = Capability::parse(" sending + receiving ,, ").expect("both");
        assert_eq!(two.roles(), [NodeRole::Receiving, NodeRole::Sending]);
        assert_eq!(two.words(), "receiving,sending");
        let cased = Capability::parse("Sending + RECEIVING").expect_err("lowercase only");
        assert!(cased.contains("called Sending, RECEIVING;"), "{cased}");

        assert_eq!(Capability::parse(""), Ok(Capability::none()));
        assert!(Capability::none().declares_no_stage());
        assert_eq!(Capability::none().words(), "");

        let refusal = Capability::parse("receiving,relay").expect_err("relay is no role");
        assert!(refusal.starts_with("REFUSED"), "{refusal}");
        assert!(refusal.contains("relay") && refusal.contains("processing"));
    }

    #[test]
    fn executing_serves_the_whole_path_and_a_role_off_the_path_serves_none() {
        let whole = Capability::parse("receiving+processing+sending").expect("the three");
        assert_eq!(whole.roles(), [NodeRole::Executing]);
        assert_eq!(whole.stages(), Stage::ALL);
        assert!(whole.executes());
        assert_eq!(Capability::serving(&Stage::ALL), whole);
        assert_eq!(
            Capability::serving(&[Stage::Send, Stage::Receive]).words(),
            "receiving,sending"
        );

        let watching = Capability::parse("monitoring,operational").expect("two roles");
        assert_eq!(watching.words(), "operational,monitoring");
        assert!(watching.declares_no_stage());
    }

    #[test]
    fn the_online_capability_rides_along() {
        let capability = Capability::of(&[NodeRole::Processing]).with_online(true);
        assert!(capability.is_online());
        assert_eq!(capability.word(), "online");
        assert_eq!(Capability::none().word(), "offline");
    }

    #[test]
    fn what_a_node_publishes_reads_back_as_what_it_declared() {
        for capability in [
            Capability::none(),
            Capability::of(&[NodeRole::Receiving]).with_online(true),
            Capability::of(&[NodeRole::Executing, NodeRole::Monitoring]),
        ] {
            let evidence = capability.evidence();
            assert_eq!(
                Capability::from_evidence(&evidence),
                Ok(capability),
                "{evidence}"
            );
        }
        assert!(
            Capability::of(&[NodeRole::Sending])
                .evidence()
                .contains("not declared in this record"),
            "the two kinds left out are said, not silent"
        );
        assert_eq!(
            Capability::none().evidence(),
            "declares no role; offline; authentication and runtime capability are not \
             declared in this record"
        );
        assert_eq!(Capability::from_evidence("alive"), Ok(Capability::none()));
    }

    #[test]
    fn a_published_declaration_reads_by_the_same_rule_as_the_flag() {
        assert_eq!(
            Capability::from_evidence("declares receiving,sending; offline; x"),
            Ok(Capability::of(&[NodeRole::Receiving, NodeRole::Sending]))
        );
        let cased = Capability::from_evidence("declares Receiving,SENDING; offline; x")
            .expect_err("lowercase only");
        assert!(cased.contains("called Receiving, SENDING;"), "{cased}");
        let refusal = Capability::from_evidence("declares receive,send; online; x")
            .expect_err("a stage word is no role");
        assert!(
            refusal.starts_with("REFUSED") && refusal.contains("receive, send"),
            "{refusal}"
        );
    }

    #[test]
    fn an_entry_names_the_node_and_what_it_was_started_with() {
        let cluster = configure::fixture::test_cluster();
        let [one, two, three] = [0, 1, 2].map(|place| cluster.node(place).name.as_str());
        let both = Capability::of(&[NodeRole::Sending, NodeRole::Receiving]);
        assert_eq!(both.entry(one), format!("{one}=receiving+sending"));
        assert_eq!(Capability::none().entry(two), two);
        assert_eq!(
            Capability::from_entry(&format!(" {one} =receiving+sending")),
            (one, Ok(both))
        );
        assert_eq!(Capability::from_entry(two), (two, Ok(Capability::none())));
        let relay = format!("{three}=relay");
        let (name, refused) = Capability::from_entry(&relay);
        assert_eq!(name, three);
        assert!(refused.expect_err("relay").starts_with("REFUSED"));
    }
}
