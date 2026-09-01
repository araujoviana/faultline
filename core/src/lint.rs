//! Rule-based resilience linting over an [`Architecture`].
//!
//! Deterministic architectural checks, each citing a principle from *Designing
//! Data-Intensive Applications* (2nd ed., Kleppmann & Riccomini). Pure graph
//! work — no I/O, no mutation. This is the "harden" beat: once the human and
//! agent have designed and stress-tested a topology, [`lint`] names the
//! anti-patterns still in it and the reason each one matters.
//!
//! Edge direction, as everywhere in the core: `a -> b` means **`a` depends on
//! `b`** (a resource's out-edges are its dependencies).

use std::collections::BTreeSet;

use serde::Serialize;

use crate::profile::ProviderProfile;
use crate::{Architecture, Resource, ResourceKind};

/// How much a [`Finding`] should worry you. Ordered: `High < Medium < Low`, so
/// sorting ascending puts the worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// A citation into *Designing Data-Intensive Applications*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Citation {
    pub source: String,
    pub chapter: String,
    pub section: String,
}

impl Citation {
    fn ddia(chapter: &str, section: &str) -> Self {
        Citation {
            source: "Designing Data-Intensive Applications, 2nd ed.".to_string(),
            chapter: chapter.to_string(),
            section: section.to_string(),
        }
    }
}

/// One resilience anti-pattern found in the architecture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Stable machine-readable rule id, e.g. `"single-az-datastore"`.
    pub rule: String,
    pub severity: Severity,
    /// The resource the finding is about, when it is about a single one.
    pub resource: Option<String>,
    /// One-line human summary.
    pub title: String,
    /// The full explanation: what is wrong and what to do about it.
    pub detail: String,
    pub citation: Citation,
}

/// Run every rule over `arch` and return the findings, worst-first and
/// deterministically ordered (severity, then rule id, then resource id).
pub fn lint(arch: &Architecture, profile: &ProviderProfile) -> Vec<Finding> {
    let mut findings = Vec::new();

    findings.extend(single_az_datastore(arch, profile));
    findings.extend(unmanaged_compute(arch));
    findings.extend(synchronous_service_coupling(arch));
    findings.extend(single_region(arch, profile));
    findings.extend(unbuffered_write_path(arch));

    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.rule.cmp(&b.rule))
            .then_with(|| a.resource.cmp(&b.resource))
    });
    findings
}

fn resource_by_id<'a>(arch: &'a Architecture, id: &str) -> Option<&'a Resource> {
    arch.resources.iter().find(|r| r.id == id)
}

fn has_inbound(arch: &Architecture, id: &str) -> bool {
    arch.edges.iter().any(|e| e.to == id)
}

fn kind_present(arch: &Architecture, kind: ResourceKind) -> bool {
    arch.resources.iter().any(|r| r.kind == kind)
}

/// **`single-az-datastore`** (high) — a datastore with no cross-zone redundancy:
/// a variant flagged as a single point of failure, or one pinned to a single
/// availability zone without an advertised failover.
///
/// DDIA Ch 6, "Handling Node Outages": a leader with no failover target is an
/// outage waiting to happen.
fn single_az_datastore(arch: &Architecture, profile: &ProviderProfile) -> Vec<Finding> {
    arch.resources
        .iter()
        .filter(|r| matches!(r.kind, ResourceKind::Database | ResourceKind::Cache))
        .filter_map(|r| {
            let variant = r
                .variant
                .as_deref()
                .and_then(|v| profile.variant(r.kind, v));
            let is_spof = variant.is_some_and(|v| v.spof);
            let zonal_no_failover = r.placement.az.is_some()
                && variant.is_none_or(|v| v.failover_seconds.is_none() && !v.spof);

            if !is_spof && !zonal_no_failover {
                return None;
            }

            let placement = r
                .placement
                .az
                .as_deref()
                .or(r.placement.region.as_deref())
                .unwrap_or("its single location");

            Some(Finding {
                rule: "single-az-datastore".to_string(),
                severity: Severity::High,
                resource: Some(r.id.clone()),
                title: format!("{} has no cross-zone redundancy", r.label),
                detail: format!(
                    "{} ({}) is a single-zone datastore: losing {placement} takes its data \
                     down with it, and everything that depends on it with that. Move to a \
                     replicated / multi-AZ variant with an automatic failover target.",
                    r.id, r.kind
                ),
                citation: Citation::ddia("Chapter 6: Replication", "Handling Node Outages"),
            })
        })
        .collect()
}

/// **`unmanaged-compute`** (medium) — a compute node that serves traffic but has
/// no provider variant, so a managed, multi-instance deployment (an autoscaling
/// group across zones) cannot be confirmed.
///
/// DDIA Ch 2, "Reliability and Fault Tolerance": tolerating a node loss means
/// having more than one node.
fn unmanaged_compute(arch: &Architecture) -> Vec<Finding> {
    arch.resources
        .iter()
        .filter(|r| {
            r.kind == ResourceKind::Compute && r.variant.is_none() && has_inbound(arch, &r.id)
        })
        .map(|r| Finding {
            rule: "unmanaged-compute".to_string(),
            severity: Severity::Medium,
            resource: Some(r.id.clone()),
            title: format!("{} serves traffic with no managed compute variant", r.label),
            detail: format!(
                "{} has inbound dependents but no compute variant is set, so horizontal \
                 redundancy across zones cannot be confirmed. Run configure-resource to pin a \
                 managed, multi-instance variant.",
                r.id
            ),
            citation: Citation::ddia(
                "Chapter 2: Defining Nonfunctional Requirements",
                "Reliability and Fault Tolerance",
            ),
        })
        .collect()
}

/// **`synchronous-service-coupling`** (medium) — one service calls another
/// directly with no queue anywhere in the design to absorb the call.
///
/// DDIA Ch 9, "Timeouts and Unbounded Delays": a synchronous call into a slow or
/// dead dependency blocks the caller with no backpressure.
fn synchronous_service_coupling(arch: &Architecture) -> Vec<Finding> {
    if kind_present(arch, ResourceKind::Queue) {
        return Vec::new();
    }

    let is_caller = |k: ResourceKind| {
        matches!(
            k,
            ResourceKind::Compute | ResourceKind::ApiGateway | ResourceKind::Functions
        )
    };
    let is_callee = |k: ResourceKind| matches!(k, ResourceKind::Compute | ResourceKind::Functions);

    arch.edges
        .iter()
        .filter_map(|e| {
            let from = resource_by_id(arch, &e.from)?;
            let to = resource_by_id(arch, &e.to)?;
            if from.id == to.id || !is_caller(from.kind) || !is_callee(to.kind) {
                return None;
            }
            Some(Finding {
                rule: "synchronous-service-coupling".to_string(),
                severity: Severity::Medium,
                resource: Some(to.id.clone()),
                title: format!("{} calls {} synchronously", from.label, to.label),
                detail: format!(
                    "{} depends directly on {} with no queue between them: a slowdown or outage \
                     in {} blocks {} with no backpressure. Put an async buffer (a queue) on \
                     calls that do not need an immediate reply.",
                    from.id, to.id, to.id, from.id
                ),
                citation: Citation::ddia(
                    "Chapter 9: The Trouble with Distributed Systems",
                    "Timeouts and Unbounded Delays",
                ),
            })
        })
        .collect()
}

/// **`single-region`** (medium) — a stateful system with two or more placed
/// resources, all in one region, and no health-checked DNS failover to a second.
///
/// DDIA Ch 6, "Multi-Region Operation": a single region is a single failure
/// domain.
fn single_region(arch: &Architecture, profile: &ProviderProfile) -> Vec<Finding> {
    let regions: BTreeSet<&str> = arch
        .resources
        .iter()
        .filter_map(|r| r.placement.region.as_deref())
        .collect();
    let placed = arch
        .resources
        .iter()
        .filter(|r| r.placement.region.is_some())
        .count();
    let has_stateful = arch
        .resources
        .iter()
        .any(|r| matches!(r.kind, ResourceKind::Database | ResourceKind::Cache));
    let has_failover_dns = arch.resources.iter().any(|r| {
        r.kind == ResourceKind::Dns
            && r.variant
                .as_deref()
                .and_then(|v| profile.variant(ResourceKind::Dns, v))
                .is_some_and(|v| v.failover_seconds.is_some())
    });

    if placed < 2 || regions.len() != 1 || !has_stateful || has_failover_dns {
        return Vec::new();
    }

    let region = regions.iter().next().copied().unwrap_or("one region");
    vec![Finding {
        rule: "single-region".to_string(),
        severity: Severity::Medium,
        resource: None,
        title: "The whole system lives in one region".to_string(),
        detail: format!(
            "Every placed resource is in {region} and there is no health-checked DNS failover \
             to a second region, so a regional outage takes everything down. Add a standby \
             region and a failover routing policy.",
        ),
        citation: Citation::ddia("Chapter 6: Replication", "Multi-Region Operation"),
    }]
}

/// **`unbuffered-write-path`** (low) — compute talks to a database with no cache
/// tier anywhere in the design.
///
/// DDIA Ch 7, "Skewed Workloads and Relieving Hot Spots": with nothing in front
/// of the database, read-heavy load and hot keys land squarely on it.
fn unbuffered_write_path(arch: &Architecture) -> Vec<Finding> {
    if kind_present(arch, ResourceKind::Cache) {
        return Vec::new();
    }

    let mut seen = BTreeSet::new();
    let mut findings = Vec::new();
    for e in &arch.edges {
        let (Some(from), Some(to)) = (resource_by_id(arch, &e.from), resource_by_id(arch, &e.to))
        else {
            continue;
        };
        if from.kind == ResourceKind::Compute
            && to.kind == ResourceKind::Database
            && seen.insert(to.id.clone())
        {
            findings.push(Finding {
                rule: "unbuffered-write-path".to_string(),
                severity: Severity::Low,
                resource: Some(to.id.clone()),
                title: format!("No cache tier in front of {}", to.label),
                detail: format!(
                    "{} reads and writes {} directly with no cache in the design: every request \
                     reaches the database, so read-heavy load has nowhere to shed and a hot key \
                     has nothing absorbing it. Consider a cache on the read path.",
                    from.id, to.id
                ),
                citation: Citation::ddia(
                    "Chapter 7: Sharding",
                    "Skewed Workloads and Relieving Hot Spots",
                ),
            });
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// alb -> ec2-asg -> rds-single-az, database pinned to us-east-1a. The
    /// `web/TESTING.md` cold-open build.
    fn single_az_demo() -> Architecture {
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

    fn rule_ids(findings: &[Finding]) -> Vec<&str> {
        findings.iter().map(|f| f.rule.as_str()).collect()
    }

    #[test]
    fn single_az_demo_flags_the_datastore_and_clears_after_hardening() {
        let p = ProviderProfile::aws();

        let before = lint(&single_az_demo(), &p);
        let db_finding = before
            .iter()
            .find(|f| f.rule == "single-az-datastore")
            .expect("single-AZ database is flagged");
        assert_eq!(db_finding.severity, Severity::High);
        assert_eq!(db_finding.resource.as_deref(), Some("database-1"));
        assert_eq!(db_finding.citation.chapter, "Chapter 6: Replication");

        // Harden: Multi-AZ, regional (zone cleared).
        let mut hardened = single_az_demo();
        hardened
            .set_variant("database-1", Some("rds-multi-az".into()))
            .unwrap();
        hardened
            .place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        let after = lint(&hardened, &p);
        assert!(
            !rule_ids(&after).contains(&"single-az-datastore"),
            "hardening clears the single-AZ finding, got {:?}",
            rule_ids(&after)
        );
    }

    #[test]
    fn fully_redundant_design_is_clean() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "edge", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        let cache = a.add_resource(ResourceKind::Cache, "sessions", 0.0, 0.0);
        let dns = a.add_resource(ResourceKind::Dns, "router", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        a.connect(&api, &db).unwrap();
        a.connect(&api, &cache).unwrap();
        for (id, variant) in [
            (&lb, "alb"),
            (&api, "ec2-asg"),
            (&db, "aurora"),
            (&cache, "elasticache"),
            (&dns, "route53"),
        ] {
            a.set_variant(id, Some(variant.to_string())).unwrap();
        }
        a.place(&db, Some("us-east-1".into()), None).unwrap();
        a.place(&cache, Some("us-east-1".into()), None).unwrap();
        a.place(&api, Some("us-east-1".into()), None).unwrap();

        assert_eq!(lint(&a, &ProviderProfile::aws()), Vec::new());
    }

    #[test]
    fn unmanaged_compute_fires_only_with_inbound_and_no_variant() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "edge", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();

        assert!(rule_ids(&lint(&a, &p)).contains(&"unmanaged-compute"));

        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        assert!(!rule_ids(&lint(&a, &p)).contains(&"unmanaged-compute"));
    }

    #[test]
    fn unmanaged_compute_ignores_a_leaf_node() {
        // No inbound edge -> not (yet) serving traffic -> not flagged.
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        a.add_resource(ResourceKind::Compute, "worker", 0.0, 0.0);
        assert!(!rule_ids(&lint(&a, &p)).contains(&"unmanaged-compute"));
    }

    #[test]
    fn synchronous_service_coupling_fires_service_to_service_without_a_queue() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let gw = a.add_resource(ResourceKind::ApiGateway, "gw", 0.0, 0.0);
        let svc = a.add_resource(ResourceKind::Functions, "checkout", 0.0, 0.0);
        a.connect(&gw, &svc).unwrap();

        let found = lint(&a, &p);
        let f = found
            .iter()
            .find(|f| f.rule == "synchronous-service-coupling")
            .expect("gw -> svc with no queue is flagged");
        assert_eq!(f.resource.as_deref(), Some("functions-1"));
        assert_eq!(
            f.citation.section, "Timeouts and Unbounded Delays",
            "cites DDIA Ch 9"
        );

        // A queue anywhere in the design silences the rule.
        a.add_resource(ResourceKind::Queue, "jobs", 0.0, 0.0);
        assert!(!rule_ids(&lint(&a, &p)).contains(&"synchronous-service-coupling"));
    }

    #[test]
    fn synchronous_service_coupling_ignores_a_plain_database_call() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.connect(&api, &db).unwrap();
        assert!(!rule_ids(&lint(&a, &p)).contains(&"synchronous-service-coupling"));
    }

    #[test]
    fn single_region_fires_for_a_stateful_multi_resource_single_region_design() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("aurora".into())).unwrap();
        a.place(&api, Some("us-east-1".into()), None).unwrap();
        a.place(&db, Some("us-east-1".into()), None).unwrap();

        let f = lint(&a, &p);
        assert!(rule_ids(&f).contains(&"single-region"));

        // A second region clears it.
        a.place("database-1", Some("eu-west-1".into()), None)
            .unwrap();
        assert!(!rule_ids(&lint(&a, &p)).contains(&"single-region"));
    }

    #[test]
    fn single_region_cleared_by_health_checked_dns() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        let dns = a.add_resource(ResourceKind::Dns, "router", 0.0, 0.0);
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("aurora".into())).unwrap();
        a.set_variant(&dns, Some("route53".into())).unwrap();
        a.place(&api, Some("us-east-1".into()), None).unwrap();
        a.place(&db, Some("us-east-1".into()), None).unwrap();

        assert!(!rule_ids(&lint(&a, &p)).contains(&"single-region"));
    }

    #[test]
    fn unbuffered_write_path_fires_without_a_cache_and_clears_with_one() {
        let p = ProviderProfile::aws();
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("aurora".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), None).unwrap();
        a.connect(&api, &db).unwrap();

        let f = lint(&a, &p);
        let hot = f
            .iter()
            .find(|f| f.rule == "unbuffered-write-path")
            .expect("compute -> database with no cache is flagged");
        assert_eq!(hot.severity, Severity::Low);
        assert_eq!(hot.citation.chapter, "Chapter 7: Sharding");

        let cache = a.add_resource(ResourceKind::Cache, "sessions", 0.0, 0.0);
        a.set_variant(&cache, Some("elasticache".into())).unwrap();
        a.connect(&api, &cache).unwrap();
        assert!(!rule_ids(&lint(&a, &p)).contains(&"unbuffered-write-path"));
    }

    #[test]
    fn findings_are_ordered_worst_first() {
        // A design that trips a high, a medium and a low at once.
        let mut a = Architecture::new();
        let gw = a.add_resource(ResourceKind::ApiGateway, "gw", 0.0, 0.0);
        let svc = a.add_resource(ResourceKind::Compute, "svc", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.connect(&gw, &svc).unwrap(); // synchronous-service-coupling (medium)
        a.connect(&svc, &db).unwrap(); // unbuffered-write-path (low)
        a.set_variant(&svc, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("rds-single-az".into())).unwrap(); // single-az-datastore (high)
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();

        let severities: Vec<Severity> = lint(&a, &ProviderProfile::aws())
            .iter()
            .map(|f| f.severity)
            .collect();
        let mut sorted = severities.clone();
        sorted.sort();
        assert_eq!(severities, sorted, "findings come back worst-first");
        assert_eq!(severities.first(), Some(&Severity::High));
    }

    #[test]
    fn empty_architecture_lints_clean() {
        assert_eq!(
            lint(&Architecture::new(), &ProviderProfile::aws()),
            Vec::new()
        );
    }

    #[test]
    fn findings_serialise_to_the_expected_shape() {
        let mut a = Architecture::new();
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();

        let json = serde_json::to_value(lint(&a, &ProviderProfile::aws())).unwrap();
        let first = &json[0];
        assert_eq!(first["rule"], "single-az-datastore");
        assert_eq!(first["severity"], "high");
        assert_eq!(first["resource"], "database-1");
        assert_eq!(first["citation"]["section"], "Handling Node Outages");
    }
}
