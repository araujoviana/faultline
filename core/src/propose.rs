//! Turn a one-line requirements sentence into a starting architecture.
//!
//! This is the "Design" beat's opening move: instead of the agent adding ten
//! resources one call at a time, [`propose`] reads a prompt like *"public web
//! app with a Postgres database that should survive an availability-zone
//! outage"* and lays down a connected, configured, placed topology the human can
//! then adjust.
//!
//! Deterministic keyword matching — no model, no I/O. The vocabulary is
//! deliberately small; the result is a *starting point*, not a final design, and
//! [`crate::lint`] is expected to still have something to say about it.

use crate::{Architecture, ResourceKind};

/// Build a starting architecture from a free-text requirements sentence.
pub fn propose(requirements: &str) -> Architecture {
    let req = requirements.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| req.contains(n));

    let region = if has(&["eu-west-1", "europe", "ireland", "frankfurt", " eu "]) {
        "eu-west-1"
    } else {
        "us-east-1"
    };
    let zone_a = format!("{region}a");

    // Resilience posture: an explicit "cheap / prototype / quick" prompt gets a
    // single-AZ datastore (and a lint finding to teach from); everything else
    // gets a replicated, regional datastore.
    let budget = has(&[
        "prototype",
        "cheap",
        "quick",
        "throwaway",
        "demo",
        "single-az",
        "minimal",
    ]);

    let serverless = has(&[
        "serverless",
        "lambda",
        "functions",
        "api gateway",
        "api-gateway",
    ]);
    let wants_cache = has(&[
        "cache",
        "read-heavy",
        "read heavy",
        "sessions",
        "hot key",
        "latency",
    ]);
    let wants_queue = has(&[
        "queue",
        "async",
        "asynchronous",
        "worker",
        "background",
        "job",
        "email",
        "notification",
        "event",
        "decouple",
    ]);
    let wants_static = has(&[
        "static",
        "spa",
        "frontend",
        "front-end",
        "assets",
        "images",
        "uploads",
        "cdn",
        "media",
    ]);
    let wants_dr = has(&[
        "multi-region",
        "multi region",
        "disaster recovery",
        " dr ",
        "region outage",
        "regional outage",
        "survive a region",
        "failover",
        "fail over",
    ]);
    let key_value = has(&["dynamodb", "dynamo", "key-value", "key value", "nosql"]);

    let mut a = Architecture::new();

    // ---- ingress ----------------------------------------------------------
    let dns = if wants_dr {
        let id = a.add_resource(ResourceKind::Dns, "router", 0.0, 0.0);
        a.set_variant(&id, Some("route53".into())).unwrap();
        Some(id)
    } else {
        None
    };

    let front = if serverless {
        let gw = a.add_resource(ResourceKind::ApiGateway, "api gateway", 0.0, 0.0);
        a.set_variant(&gw, Some("apigw-http".into())).unwrap();
        a.place(&gw, Some(region.into()), None).unwrap();
        gw
    } else {
        let lb = a.add_resource(ResourceKind::LoadBalancer, "load balancer", 0.0, 0.0);
        a.set_variant(&lb, Some("alb".into())).unwrap();
        a.place(&lb, Some(region.into()), None).unwrap();
        lb
    };
    if let Some(dns) = &dns {
        a.connect(dns, &front).unwrap();
    }

    // ---- compute --------------------------------------------------------
    let app = if serverless {
        let f = a.add_resource(ResourceKind::Functions, "api", 0.0, 0.0);
        a.set_variant(&f, Some("lambda".into())).unwrap();
        a.place(&f, Some(region.into()), None).unwrap();
        f
    } else {
        let c = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.set_variant(&c, Some("ec2-asg".into())).unwrap();
        a.place(&c, Some(region.into()), None).unwrap();
        c
    };
    a.connect(&front, &app).unwrap();

    // ---- data ----------------------------------------------------------
    let db = a.add_resource(ResourceKind::Database, "primary datastore", 0.0, 0.0);
    if key_value {
        a.set_variant(&db, Some("dynamodb".into())).unwrap();
        a.place(&db, Some(region.into()), None).unwrap();
    } else if budget {
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some(region.into()), Some(zone_a.clone()))
            .unwrap();
    } else {
        a.set_variant(&db, Some("rds-multi-az".into())).unwrap();
        a.place(&db, Some(region.into()), None).unwrap();
    }
    a.connect(&app, &db).unwrap();

    // ---- optional tiers ------------------------------------------------
    if wants_cache {
        let cache = a.add_resource(ResourceKind::Cache, "cache", 0.0, 0.0);
        a.set_variant(&cache, Some("elasticache".into())).unwrap();
        a.place(&cache, Some(region.into()), None).unwrap();
        a.connect(&app, &cache).unwrap();
    }

    if wants_queue {
        let queue = a.add_resource(ResourceKind::Queue, "job queue", 0.0, 0.0);
        a.set_variant(&queue, Some("sqs".into())).unwrap();
        a.place(&queue, Some(region.into()), None).unwrap();
        a.connect(&app, &queue).unwrap();

        let worker = a.add_resource(ResourceKind::Functions, "worker", 0.0, 0.0);
        a.set_variant(&worker, Some("lambda".into())).unwrap();
        a.place(&worker, Some(region.into()), None).unwrap();
        a.connect(&worker, &queue).unwrap();
        a.connect(&worker, &db).unwrap();
    }

    if wants_static {
        let store = a.add_resource(ResourceKind::ObjectStore, "asset store", 0.0, 0.0);
        a.set_variant(&store, Some("s3".into())).unwrap();
        a.place(&store, Some(region.into()), None).unwrap();

        let cdn = a.add_resource(ResourceKind::Cdn, "cdn", 0.0, 0.0);
        a.set_variant(&cdn, Some("cloudfront".into())).unwrap();
        a.connect(&cdn, &store).unwrap();
        if let Some(dns) = &dns {
            a.connect(dns, &cdn).unwrap();
        }
    }

    layout(&mut a);
    a
}

/// Assign canvas positions by dependency tier: ingress at the top, data at the
/// bottom, siblings spread horizontally. Keeps a freshly proposed design
/// readable before anyone drags it around.
fn layout(a: &mut Architecture) {
    fn tier(kind: ResourceKind) -> i32 {
        match kind {
            ResourceKind::Dns | ResourceKind::Cdn => 0,
            ResourceKind::LoadBalancer | ResourceKind::ApiGateway => 1,
            ResourceKind::Compute | ResourceKind::Functions => 2,
            ResourceKind::Cache | ResourceKind::Queue => 3,
            ResourceKind::Database | ResourceKind::ObjectStore => 4,
        }
    }

    const X0: f64 = 40.0;
    const Y0: f64 = 24.0;
    const DX: f64 = 168.0;
    const DY: f64 = 104.0;

    let ids: Vec<(String, ResourceKind)> =
        a.resources.iter().map(|r| (r.id.clone(), r.kind)).collect();

    for t in 0..=4 {
        let mut col = 0.0;
        for (id, _kind) in ids.iter().filter(|(_, k)| tier(*k) == t) {
            if let Some(r) = a.resources.iter_mut().find(|r| &r.id == id) {
                r.x = X0 + col * DX;
                r.y = Y0 + f64::from(t) * DY;
            }
            col += 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::{lint, Severity};
    use crate::profile::ProviderProfile;

    fn kinds(a: &Architecture) -> Vec<ResourceKind> {
        a.resources.iter().map(|r| r.kind).collect()
    }

    #[test]
    fn plain_web_app_gets_a_sound_three_tier_stack() {
        let a = propose("public web app with a database");
        assert!(kinds(&a).contains(&ResourceKind::LoadBalancer));
        assert!(kinds(&a).contains(&ResourceKind::Compute));
        assert!(kinds(&a).contains(&ResourceKind::Database));
        // Every resource is connected into one graph.
        assert_eq!(a.edges.len(), 2);
        // Multi-AZ datastore by default → no high-severity findings.
        let findings = lint(&a, &ProviderProfile::aws());
        assert!(
            !findings.iter().any(|f| f.severity == Severity::High),
            "default proposal has no high findings, got {:?}",
            findings.iter().map(|f| &f.rule).collect::<Vec<_>>()
        );
    }

    #[test]
    fn budget_prompt_produces_a_single_az_datastore_to_teach_from() {
        let a = propose("quick prototype web app, single database, cheap");
        let db = a
            .resources
            .iter()
            .find(|r| r.kind == ResourceKind::Database)
            .unwrap();
        assert_eq!(db.variant.as_deref(), Some("rds-single-az"));
        assert_eq!(db.placement.az.as_deref(), Some("us-east-1a"));
        let findings = lint(&a, &ProviderProfile::aws());
        assert!(findings.iter().any(|f| f.rule == "single-az-datastore"));
    }

    #[test]
    fn keywords_pull_in_the_matching_tiers() {
        let a = propose(
            "read-heavy web app that also processes background jobs and serves static assets",
        );
        let ks = kinds(&a);
        assert!(ks.contains(&ResourceKind::Cache));
        assert!(ks.contains(&ResourceKind::Queue));
        assert!(ks.contains(&ResourceKind::ObjectStore));
        assert!(ks.contains(&ResourceKind::Cdn));
    }

    #[test]
    fn serverless_prompt_swaps_lb_compute_for_gateway_functions() {
        let a = propose("serverless API with a key-value store");
        let ks = kinds(&a);
        assert!(ks.contains(&ResourceKind::ApiGateway));
        assert!(ks.contains(&ResourceKind::Functions));
        assert!(!ks.contains(&ResourceKind::LoadBalancer));
        let db = a
            .resources
            .iter()
            .find(|r| r.kind == ResourceKind::Database)
            .unwrap();
        assert_eq!(db.variant.as_deref(), Some("dynamodb"));
    }

    #[test]
    fn dr_prompt_adds_health_checked_dns_and_clears_single_region() {
        let a = propose("web app with a database that must survive a region outage");
        assert!(kinds(&a).contains(&ResourceKind::Dns));
        let findings = lint(&a, &ProviderProfile::aws());
        assert!(!findings.iter().any(|f| f.rule == "single-region"));
    }

    #[test]
    fn region_is_read_from_the_prompt() {
        let a = propose("web app hosted in europe");
        assert!(a
            .resources
            .iter()
            .all(|r| r.placement.region.as_deref() == Some("eu-west-1") || r.kind.is_global()));
    }

    #[test]
    fn layout_places_ingress_above_data() {
        let a = propose("web app with a database and a cache");
        let lb = a
            .resources
            .iter()
            .find(|r| r.kind == ResourceKind::LoadBalancer)
            .unwrap();
        let db = a
            .resources
            .iter()
            .find(|r| r.kind == ResourceKind::Database)
            .unwrap();
        assert!(lb.y < db.y);
    }

    #[test]
    fn result_is_deterministic() {
        assert_eq!(
            propose("read-heavy web app with async jobs"),
            propose("read-heavy web app with async jobs")
        );
    }
}
