//! Deterministic failure analysis over an [`Architecture`].
//!
//! Two questions, both pure graph work:
//!
//! * **Blast radius** — given a set of resources knocked out (e.g. everything in
//!   a failed availability zone), what else becomes unavailable, and what merely
//!   degrades?
//! * **SPOFs** — which resources are single points of failure by construction
//!   (a `spof` variant in the active [`ProviderProfile`]), and what do they take
//!   down with them?
//!
//! Edge direction: `a -> b` means **`a` depends on `b`**. A resource's out-edges
//! are its dependencies; they are grouped by the depended-on [`ResourceKind`], so
//! two compute nodes behind one load balancer read as one redundant group.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::profile::ProviderProfile;
use crate::Architecture;

/// The outcome of a simulated failure.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlastReport {
    /// Human-readable description of what failed, e.g. `"AZ us-east-1a"`.
    pub target: String,
    /// Fully unavailable (the seed plus everything transitively cut off).
    pub down: Vec<String>,
    /// Still serving, but impaired — lost a redundant peer, or mid-failover.
    pub degraded: Vec<String>,
    /// Unaffected.
    pub healthy: Vec<String>,
    /// Plain-language remarks tied to `down` / `degraded` entries.
    pub notes: Vec<String>,
}

/// A single point of failure and what it orphans.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spof {
    pub id: String,
    pub orphans: Vec<String>,
}

/// Resource ids directly knocked out by losing availability zone `az`: anything
/// explicitly placed in it.
pub fn az_failure_seed(arch: &Architecture, az: &str) -> Vec<String> {
    arch.resources
        .iter()
        .filter(|r| r.placement.az.as_deref() == Some(az))
        .map(|r| r.id.clone())
        .collect()
}

/// Resource ids knocked out by losing the whole region: everything placed in it
/// except globally distributed services (CDN, DNS). A copy of the stack in
/// another region is not in the seed, so it survives.
pub fn region_failure_seed(arch: &Architecture, region: &str) -> Vec<String> {
    arch.resources
        .iter()
        .filter(|r| !r.kind.is_global() && r.placement.region.as_deref() == Some(region))
        .map(|r| r.id.clone())
        .collect()
}

/// Propagate a failure from `down_seed` through the dependency graph.
///
/// `failed_region`, when set, enables the "mid-failover" degradation note for
/// regional resources whose variant advertises a `failover_seconds`.
pub fn blast_radius(
    arch: &Architecture,
    profile: &ProviderProfile,
    down_seed: &[String],
    failed_region: Option<&str>,
    target: &str,
) -> BlastReport {
    let known: BTreeSet<&str> = arch.resources.iter().map(|r| r.id.as_str()).collect();
    let mut down: BTreeSet<String> = down_seed
        .iter()
        .filter(|id| known.contains(id.as_str()))
        .cloned()
        .collect();

    // Fixpoint: a resource with dependencies goes down once some dependency
    // group is entirely down.
    loop {
        let mut changed = false;
        for r in &arch.resources {
            if down.contains(&r.id) {
                continue;
            }
            let groups = deps_by_kind(arch, &r.id);
            if !groups.is_empty()
                && groups
                    .values()
                    .any(|ids| ids.iter().all(|t| down.contains(*t)))
            {
                down.insert(r.id.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut degraded: BTreeSet<String> = BTreeSet::new();
    let mut notes: Vec<String> = Vec::new();

    for r in &arch.resources {
        if down.contains(&r.id) {
            continue;
        }

        // Lost some, but not all, of a redundant dependency group.
        if let Some((kind, n_down, total)) = partially_down_group(arch, &r.id, &down) {
            degraded.insert(r.id.clone());
            notes.push(format!(
                "{} lost {n_down} of {total} {kind} dependencies",
                r.label
            ));
            continue;
        }

        // Regional resource in the failed region that fails over rather than
        // staying perfectly available.
        if let Some(region) = failed_region {
            if r.placement.region.as_deref() == Some(region) {
                if let Some(secs) = r
                    .variant
                    .as_deref()
                    .and_then(|v| profile.variant(r.kind, v))
                    .and_then(|v| v.failover_seconds)
                {
                    degraded.insert(r.id.clone());
                    notes.push(format!("{} may briefly fail over (~{secs}s)", r.label));
                }
            }
        }
    }

    let healthy: Vec<String> = arch
        .resources
        .iter()
        .map(|r| r.id.clone())
        .filter(|id| !down.contains(id) && !degraded.contains(id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    BlastReport {
        target: target.to_string(),
        down: down.into_iter().collect(),
        degraded: degraded.into_iter().collect(),
        healthy,
        notes,
    }
}

/// Every resource whose variant is a single point of failure, with the resources
/// it would orphan.
pub fn spofs(arch: &Architecture, profile: &ProviderProfile) -> Vec<Spof> {
    arch.resources
        .iter()
        .filter(|r| {
            r.variant
                .as_deref()
                .and_then(|v| profile.variant(r.kind, v))
                .is_some_and(|v| v.spof)
        })
        .map(|r| {
            let report = blast_radius(arch, profile, std::slice::from_ref(&r.id), None, &r.id);
            Spof {
                id: r.id.clone(),
                orphans: report.down.into_iter().filter(|id| id != &r.id).collect(),
            }
        })
        .collect()
}

/// A resource's dependencies grouped by the depended-on kind slug (sorted, for
/// deterministic output).
fn deps_by_kind<'a>(arch: &'a Architecture, id: &str) -> BTreeMap<&'static str, Vec<&'a str>> {
    let mut groups: BTreeMap<&'static str, Vec<&'a str>> = BTreeMap::new();
    for e in &arch.edges {
        if e.from != id {
            continue;
        }
        if let Some(target) = arch.resources.iter().find(|r| r.id == e.to) {
            groups
                .entry(target.kind.slug())
                .or_default()
                .push(target.id.as_str());
        }
    }
    groups
}

fn partially_down_group(
    arch: &Architecture,
    id: &str,
    down: &BTreeSet<String>,
) -> Option<(&'static str, usize, usize)> {
    for (kind, ids) in deps_by_kind(arch, id) {
        let n_down = ids.iter().filter(|t| down.contains(**t)).count();
        if n_down > 0 && n_down < ids.len() {
            return Some((kind, n_down, ids.len()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceKind;

    /// LB -> compute -> database, database is single-AZ in us-east-1a.
    fn chain() -> Architecture {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "edge", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        a.connect(&api, &db).unwrap();
        a.set_variant(&lb, Some("alb".into())).unwrap();
        a.place(&lb, Some("us-east-1".into()), None).unwrap();
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.place(&api, Some("us-east-1".into()), None).unwrap();
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        a
    }

    #[test]
    fn az_failure_takes_down_the_whole_chain() {
        let a = chain();
        let p = ProviderProfile::aws();
        let seed = az_failure_seed(&a, "us-east-1a");
        let r = blast_radius(&a, &p, &seed, Some("us-east-1"), "AZ us-east-1a");
        assert_eq!(r.down, ["compute-1", "database-1", "load-balancer-1"]);
        assert!(r.degraded.is_empty());
        assert!(r.healthy.is_empty());
    }

    #[test]
    fn multi_az_database_degrades_instead_of_dying() {
        let mut a = chain();
        let p = ProviderProfile::aws();
        a.set_variant("database-1", Some("rds-multi-az".into()))
            .unwrap();
        a.place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        let seed = az_failure_seed(&a, "us-east-1a");
        let r = blast_radius(&a, &p, &seed, Some("us-east-1"), "AZ us-east-1a");
        assert!(r.down.is_empty());
        assert_eq!(r.degraded, ["database-1"]);
        assert_eq!(r.healthy, ["compute-1", "load-balancer-1"]);
        assert!(r.notes.iter().any(|n| n.contains("~90s")));
    }

    #[test]
    fn dead_dependency_group_downs_the_dependent_even_with_a_live_sibling_group() {
        // compute depends on BOTH a database (single-AZ) and a cache (multi-AZ).
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        let cache = a.add_resource(ResourceKind::Cache, "sessions", 0.0, 0.0);
        a.connect(&api, &db).unwrap();
        a.connect(&api, &cache).unwrap();
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        a.set_variant(&cache, Some("elasticache".into())).unwrap();
        a.place(&cache, Some("us-east-1".into()), None).unwrap();

        let p = ProviderProfile::aws();
        let seed = az_failure_seed(&a, "us-east-1a");
        let r = blast_radius(&a, &p, &seed, Some("us-east-1"), "AZ us-east-1a");
        assert_eq!(r.down, ["compute-1", "database-1"]);
    }

    #[test]
    fn losing_one_of_two_backends_degrades_the_load_balancer() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "edge", 0.0, 0.0);
        let a1 = a.add_resource(ResourceKind::Compute, "api-a", 0.0, 0.0);
        let a2 = a.add_resource(ResourceKind::Compute, "api-b", 0.0, 0.0);
        a.connect(&lb, &a1).unwrap();
        a.connect(&lb, &a2).unwrap();
        a.place(&a1, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        a.place(&a2, Some("us-east-1".into()), Some("us-east-1b".into()))
            .unwrap();

        let p = ProviderProfile::aws();
        let seed = az_failure_seed(&a, "us-east-1a");
        let r = blast_radius(&a, &p, &seed, Some("us-east-1"), "AZ us-east-1a");
        assert_eq!(r.down, ["compute-1"]);
        assert_eq!(r.degraded, ["load-balancer-1"]);
        assert!(r.notes.iter().any(|n| n.contains("lost 1 of 2")));
    }

    #[test]
    fn region_failure_downs_the_local_stack_but_not_a_second_region() {
        // us-east-1 stack, plus a compute in eu-west-1 fronted by global DNS.
        let mut a = chain();
        let dns = a.add_resource(ResourceKind::Dns, "router", 0.0, 0.0);
        let dr = a.add_resource(ResourceKind::Compute, "api-dr", 0.0, 0.0);
        a.set_variant(&dns, Some("route53".into())).unwrap();
        a.set_variant(&dr, Some("ec2-asg".into())).unwrap();
        a.place(&dr, Some("eu-west-1".into()), None).unwrap();
        a.connect(&dns, "compute-1").unwrap();
        a.connect(&dns, &dr).unwrap();

        let p = ProviderProfile::aws();
        let seed = region_failure_seed(&a, "us-east-1");
        let r = blast_radius(&a, &p, &seed, Some("us-east-1"), "region us-east-1");
        assert!(r.down.contains(&"compute-1".to_string()));
        assert!(r.down.contains(&"database-1".to_string()));
        assert!(r.down.contains(&"load-balancer-1".to_string()));
        // The eu-west-1 compute survives; the global DNS stays up but degraded
        // (it lost one of its two backends and is failing traffic over).
        assert!(r.healthy.contains(&"compute-2".to_string()));
        assert!(r.degraded.contains(&"dns-1".to_string()));
        assert!(!r.down.contains(&"dns-1".to_string()));
    }

    #[test]
    fn region_failure_seed_excludes_global_services() {
        let mut a = Architecture::new();
        let cdn = a.add_resource(ResourceKind::Cdn, "edge", 0.0, 0.0);
        a.set_variant(&cdn, Some("cloudfront".into())).unwrap();
        a.place(&cdn, Some("us-east-1".into()), None).unwrap();
        assert!(region_failure_seed(&a, "us-east-1").is_empty());
    }

    #[test]
    fn spof_scan_finds_the_single_az_database_and_its_orphans() {
        let a = chain();
        let p = ProviderProfile::aws();
        let found = spofs(&a, &p);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "database-1");
        assert_eq!(found[0].orphans, ["compute-1", "load-balancer-1"]);
    }

    #[test]
    fn spof_scan_is_empty_for_an_all_redundant_design() {
        let mut a = chain();
        let p = ProviderProfile::aws();
        a.set_variant("database-1", Some("aurora".into())).unwrap();
        a.place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        assert!(spofs(&a, &p).is_empty());
    }
}
