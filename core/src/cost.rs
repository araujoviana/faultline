//! Rough monthly cost estimate: each configured resource contributes its
//! variant's bundled `monthly_usd` snapshot from the active [`ProviderProfile`].
//! An order-of-magnitude figure for comparing designs, not a bill.

use serde::Serialize;

use crate::profile::ProviderProfile;
use crate::Architecture;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostLine {
    pub resource: String,
    pub label: String,
    pub variant: String,
    pub monthly_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostReport {
    pub total_monthly_usd: f64,
    pub lines: Vec<CostLine>,
    /// Resource ids with no variant, or a variant the profile does not price.
    pub unpriced: Vec<String>,
}

/// Variants already multi-AZ by construction, so a regional placement doesn't
/// double their cost.
fn already_redundant(variant_id: &str) -> bool {
    matches!(
        variant_id,
        "rds-multi-az"
            | "aurora"
            | "aurora-serverless"
            | "dynamodb"
            | "elasticache"
            | "s3"
            | "alb"
            | "nlb"
            | "apigw-http"
            | "sqs"
            | "cloudfront"
            | "route53"
            | "lambda"
            | "fargate"
    )
}

/// Estimate the current design's monthly cost.
pub fn estimate(arch: &Architecture, profile: &ProviderProfile) -> CostReport {
    let mut lines = Vec::new();
    let mut unpriced = Vec::new();

    for r in &arch.resources {
        let Some(variant_id) = r.variant.as_deref() else {
            unpriced.push(r.id.clone());
            continue;
        };
        let Some(price) = profile
            .variant(r.kind, variant_id)
            .and_then(|v| v.monthly_usd)
        else {
            unpriced.push(r.id.clone());
            continue;
        };

        // Regional deployment of a not-already-redundant variant ≈ 2 copies.
        let regional = r.placement.region.is_some() && r.placement.az.is_none();
        let multiplier = if regional && !already_redundant(variant_id) {
            2.0
        } else {
            1.0
        };

        lines.push(CostLine {
            resource: r.id.clone(),
            label: r.label.clone(),
            variant: variant_id.to_string(),
            monthly_usd: round2(price * multiplier),
        });
    }

    lines.sort_by(|a, b| {
        b.monthly_usd
            .partial_cmp(&a.monthly_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.resource.cmp(&b.resource))
    });
    unpriced.sort();

    let total = round2(lines.iter().map(|l| l.monthly_usd).sum());
    CostReport {
        total_monthly_usd: total,
        lines,
        unpriced,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceKind;

    fn stack() -> Architecture {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "alb", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.set_variant(&lb, Some("alb".into())).unwrap();
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("rds-single-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        a
    }

    #[test]
    fn sums_priced_variants_worst_first() {
        let r = estimate(&stack(), &ProviderProfile::aws());
        // 23 (alb) + 62 (ec2-asg) + 78 (rds-single-az, zonal) = 163
        assert_eq!(r.total_monthly_usd, 163.0);
        assert_eq!(r.lines[0].resource, "database-1");
        assert!(r.unpriced.is_empty());
    }

    #[test]
    fn regional_placement_of_a_plain_variant_doubles_it() {
        let mut a = stack();
        a.place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        let r = estimate(&a, &ProviderProfile::aws());
        let db = r.lines.iter().find(|l| l.resource == "database-1").unwrap();
        assert_eq!(db.monthly_usd, 156.0);
    }

    #[test]
    fn multi_az_variant_is_not_doubled_when_regional() {
        let mut a = stack();
        a.set_variant("database-1", Some("rds-multi-az".into()))
            .unwrap();
        a.place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        let r = estimate(&a, &ProviderProfile::aws());
        let db = r.lines.iter().find(|l| l.resource == "database-1").unwrap();
        assert_eq!(db.monthly_usd, 156.0);
    }

    #[test]
    fn unconfigured_resource_is_listed_as_unpriced() {
        let mut a = Architecture::new();
        a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let r = estimate(&a, &ProviderProfile::aws());
        assert_eq!(r.unpriced, ["compute-1"]);
        assert_eq!(r.total_monthly_usd, 0.0);
    }

    #[test]
    fn empty_design_costs_nothing() {
        let r = estimate(&Architecture::new(), &ProviderProfile::aws());
        assert_eq!(r.total_monthly_usd, 0.0);
        assert!(r.lines.is_empty());
    }

    #[test]
    fn report_serialises_to_expected_shape() {
        let json = serde_json::to_value(estimate(&stack(), &ProviderProfile::aws())).unwrap();
        assert!(json["total_monthly_usd"].is_number());
        assert_eq!(json["lines"][0]["resource"], "database-1");
        assert!(json["unpriced"].is_array());
    }
}
