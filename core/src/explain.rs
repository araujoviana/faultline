//! Plain-language explanation of one thing on the canvas — a resource or a
//! dependency edge.
//!
//! This is the teaching surface: the human (or their agent) selects something
//! and asks "why is this here / what does it do / what happens if it fails".
//! Pure read: [`explain`] reads the graph and the active profile and returns a
//! structured [`Explanation`]; it changes nothing.
//!
//! Edge convention, as everywhere in the core: `a -> b` means **`a` depends on
//! `b`** (a resource's out-edges are its dependencies).

use serde::Serialize;

use crate::analysis::blast_radius;
use crate::profile::ProviderProfile;
use crate::{Architecture, ResourceKind};

/// A structured explanation of a selected resource or edge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Explanation {
    /// What is being explained, e.g. `"database-1 (orders)"` or
    /// `"compute-1 → database-1"`.
    pub subject: String,
    /// `"resource"` or `"dependency"`.
    pub selection_kind: String,
    /// One-line description of the role this plays in the design.
    pub summary: String,
    /// Resource ids this depends on (its out-edges), with labels.
    pub depends_on: Vec<String>,
    /// Resource ids that depend on this (its in-edges), with labels.
    pub depended_on_by: Vec<String>,
    /// Everything that goes fully unavailable if this resource is lost
    /// (transitive), excluding the resource itself.
    pub takes_down: Vec<String>,
    /// Observations about the current configuration and one architectural
    /// principle that applies, each a full sentence.
    pub notes: Vec<String>,
}

/// The vendor-neutral role each [`ResourceKind`] plays.
fn role(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Compute => {
            "Runs your application code. Tolerating the loss of one instance means running more than one, across zones."
        }
        ResourceKind::Database => {
            "The system of record. Its replication and failover posture sets the whole design's data-loss and downtime risk."
        }
        ResourceKind::Queue => {
            "An asynchronous buffer between a producer and a consumer, so a slow or dead consumer applies backpressure instead of blocking the caller."
        }
        ResourceKind::LoadBalancer => {
            "Spreads inbound traffic across healthy compute instances and takes unhealthy ones out of rotation."
        }
        ResourceKind::ObjectStore => {
            "Durable blob storage, replicated across zones within a region by default."
        }
        ResourceKind::Cache => {
            "A fast read-path tier in front of a slower datastore. Absorbs read-heavy load and hot keys."
        }
        ResourceKind::Cdn => {
            "Serves static assets from edge locations close to users. Globally distributed, so no single zone or region is on its critical path."
        }
        ResourceKind::Dns => {
            "Resolves names to addresses. With health checks it can shift traffic away from a failed endpoint or region."
        }
        ResourceKind::Functions => {
            "Event-driven compute with no server to manage. Scales with load, down to zero."
        }
        ResourceKind::ApiGateway => {
            "The front door for an API: routing, authentication and rate-limiting before a request reaches compute."
        }
    }
}

/// One architectural principle relevant to this kind, cited to *Designing
/// Data-Intensive Applications* (2nd ed.).
fn principle(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Database => {
            "DDIA Ch 6 (\"Handling Node Outages\"): a leader with no automatic failover target is an outage waiting to happen — prefer a replicated, multi-AZ variant."
        }
        ResourceKind::Compute | ResourceKind::Functions => {
            "DDIA Ch 2 (\"Reliability and Fault Tolerance\"): tolerating a node loss requires more than one node."
        }
        ResourceKind::Cache => {
            "DDIA Ch 7 (\"Skewed Workloads\"): a cache on the read path is what a hot key lands on instead of the database."
        }
        ResourceKind::Queue => {
            "DDIA Ch 9 (\"Timeouts and Unbounded Delays\"): an async buffer stops a slow dependency from blocking its caller."
        }
        ResourceKind::LoadBalancer | ResourceKind::ApiGateway => {
            "DDIA Ch 2: a component that fronts others is only as available as its own redundancy — keep it multi-AZ."
        }
        ResourceKind::Dns => {
            "DDIA Ch 6 (\"Multi-Region Operation\"): health-checked DNS is the routing layer that makes a standby region reachable."
        }
        ResourceKind::ObjectStore | ResourceKind::Cdn => {
            "DDIA Ch 6 (\"Replication\"): replication across zones is what turns a single-copy store into a durable one."
        }
    }
}

fn with_label(arch: &Architecture, id: &str) -> String {
    let label = arch
        .resources
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.label.as_str())
        .unwrap_or(id);
    format!("{id} ({label})")
}

/// Explain one resource id, or one edge written as `"from->to"` / `"from → to"`.
pub fn explain(arch: &Architecture, profile: &ProviderProfile, selection: &str) -> Explanation {
    let sel = selection.trim();

    // Edge form: "a->b" or "a → b".
    let edge_parts: Option<(&str, &str)> = sel
        .split_once("->")
        .or_else(|| sel.split_once('→'))
        .map(|(a, b)| (a.trim(), b.trim()));

    if let Some((from, to)) = edge_parts {
        return explain_edge(arch, from, to);
    }

    explain_resource(arch, profile, sel)
}

fn explain_edge(arch: &Architecture, from: &str, to: &str) -> Explanation {
    let exists = arch.edges.iter().any(|e| e.from == from && e.to == to);
    let from_res = arch.resources.iter().find(|r| r.id == from);
    let to_res = arch.resources.iter().find(|r| r.id == to);

    let mut notes = Vec::new();
    if !exists {
        notes.push(format!(
            "There is no {from} → {to} dependency in the current design."
        ));
    }

    let summary = match (from_res, to_res) {
        (Some(f), Some(t)) => {
            let buffered = arch.resources.iter().any(|r| r.kind == ResourceKind::Queue);
            let sync_note = if matches!(
                f.kind,
                ResourceKind::Compute | ResourceKind::ApiGateway | ResourceKind::Functions
            ) && matches!(
                t.kind,
                ResourceKind::Compute | ResourceKind::Functions
            ) && !buffered
            {
                " This is a synchronous service-to-service call with no queue to absorb it: a slowdown in the callee propagates straight back to the caller."
            } else {
                ""
            };
            format!(
                "{} depends on {}: {} is on {}'s critical path, so {} is unavailable whenever {} is.{}",
                f.label, t.label, t.label, f.label, f.label, t.label, sync_note
            )
        }
        _ => format!("{from} → {to} references a resource that is not on the canvas."),
    };

    Explanation {
        subject: format!("{from} → {to}"),
        selection_kind: "dependency".to_string(),
        summary,
        depends_on: Vec::new(),
        depended_on_by: Vec::new(),
        takes_down: Vec::new(),
        notes,
    }
}

fn explain_resource(arch: &Architecture, profile: &ProviderProfile, id: &str) -> Explanation {
    let Some(resource) = arch.resources.iter().find(|r| r.id == id) else {
        return Explanation {
            subject: id.to_string(),
            selection_kind: "resource".to_string(),
            summary: format!("There is no resource {id} on the canvas."),
            depends_on: Vec::new(),
            depended_on_by: Vec::new(),
            takes_down: Vec::new(),
            notes: Vec::new(),
        };
    };

    let depends_on: Vec<String> = arch
        .edges
        .iter()
        .filter(|e| e.from == id)
        .map(|e| with_label(arch, &e.to))
        .collect();
    let depended_on_by: Vec<String> = arch
        .edges
        .iter()
        .filter(|e| e.to == id)
        .map(|e| with_label(arch, &e.from))
        .collect();

    let report = blast_radius(arch, profile, std::slice::from_ref(&resource.id), None, id);
    let takes_down: Vec<String> = report
        .down
        .into_iter()
        .filter(|d| d != id)
        .map(|d| with_label(arch, &d))
        .collect();

    let mut notes = Vec::new();

    match resource.variant.as_deref() {
        None => notes.push(format!(
            "No provider variant is set, so failure analysis treats {id} generically. Run configure-resource to pin a concrete service.",
        )),
        Some(v) => match profile.variant(resource.kind, v) {
            Some(variant) if variant.spof => notes.push(format!(
                "Its variant ({}) is a single point of failure by construction: no built-in redundancy.",
                variant.display_name
            )),
            Some(variant) => {
                let fo = variant
                    .failover_seconds
                    .map(|s| format!(", failing over in about {s}s"))
                    .unwrap_or_default();
                notes.push(format!(
                    "Its variant is {}{fo}.",
                    variant.display_name
                ));
            }
            None => notes.push(format!(
                "Its variant \"{v}\" is not in the {} profile.",
                profile.display_name
            )),
        },
    }

    match (&resource.placement.region, &resource.placement.az) {
        (Some(region), Some(az)) => notes.push(format!(
            "Placed in a single zone ({az}) of {region}: losing that one zone takes it with it.",
        )),
        (Some(region), None) => notes.push(format!(
            "Placed regionally in {region} (spread across that region's zones).",
        )),
        _ if resource.kind.is_global() => notes
            .push("Global service — it has no single-zone or single-region footprint.".to_string()),
        _ => notes.push("Not placed in any region yet.".to_string()),
    }

    notes.push(principle(resource.kind).to_string());

    Explanation {
        subject: format!("{id} ({})", resource.label),
        selection_kind: "resource".to_string(),
        summary: role(resource.kind).to_string(),
        depends_on,
        depended_on_by,
        takes_down,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// alb -> ec2-asg -> rds-single-az, DB pinned to us-east-1a.
    fn demo() -> Architecture {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "alb", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        a.connect(&api, &db).unwrap();
        a.set_variant(&lb, Some("alb".into())).unwrap();
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        a
    }

    #[test]
    fn explains_a_resource_with_role_deps_and_blast_radius() {
        let a = demo();
        let p = ProviderProfile::aws();
        let e = explain(&a, &p, "database-1");
        assert_eq!(e.subject, "database-1 (orders)");
        assert_eq!(e.selection_kind, "resource");
        assert!(e.summary.contains("system of record"));
        assert_eq!(e.depended_on_by, ["compute-1 (api)"]);
        assert!(e.depends_on.is_empty());
        // Losing the single-AZ DB takes the whole chain with it.
        assert_eq!(e.takes_down, ["compute-1 (api)", "load-balancer-1 (alb)"]);
        assert!(e
            .notes
            .iter()
            .any(|n| n.contains("single point of failure")));
        assert!(e.notes.iter().any(|n| n.contains("single zone")));
        assert!(e.notes.iter().any(|n| n.contains("DDIA Ch 6")));
    }

    #[test]
    fn explains_an_edge_and_flags_the_synchronous_call() {
        let mut a = Architecture::new();
        let gw = a.add_resource(ResourceKind::ApiGateway, "gw", 0.0, 0.0);
        let svc = a.add_resource(ResourceKind::Functions, "checkout", 0.0, 0.0);
        a.connect(&gw, &svc).unwrap();
        let p = ProviderProfile::aws();

        let e = explain(&a, &p, "api-gateway-1 -> functions-1");
        assert_eq!(e.selection_kind, "dependency");
        assert!(e.summary.contains("critical path"));
        assert!(e.summary.contains("synchronous service-to-service call"));
    }

    #[test]
    fn edge_that_does_not_exist_is_called_out() {
        let a = demo();
        let p = ProviderProfile::aws();
        let e = explain(&a, &p, "load-balancer-1->database-1");
        assert!(e
            .notes
            .iter()
            .any(|n| n.contains("no load-balancer-1 → database-1 dependency")));
    }

    #[test]
    fn unknown_resource_is_handled() {
        let a = demo();
        let p = ProviderProfile::aws();
        let e = explain(&a, &p, "ghost-9");
        assert!(e.summary.contains("no resource ghost-9"));
    }

    #[test]
    fn unconfigured_resource_says_so() {
        let mut a = Architecture::new();
        a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let p = ProviderProfile::aws();
        let e = explain(&a, &p, "compute-1");
        assert!(e
            .notes
            .iter()
            .any(|n| n.contains("No provider variant is set")));
        assert!(e
            .notes
            .iter()
            .any(|n| n.contains("Not placed in any region")));
    }

    #[test]
    fn explanation_serialises_to_expected_shape() {
        let a = demo();
        let p = ProviderProfile::aws();
        let json = serde_json::to_value(explain(&a, &p, "compute-1")).unwrap();
        assert_eq!(json["selection_kind"], "resource");
        assert!(json["depends_on"].is_array());
        assert!(json["takes_down"].is_array());
    }
}
