//! Provider profiles: the concrete, per-cloud data layer that sits on top of the
//! vendor-neutral graph model.
//!
//! A [`ProviderProfile`] maps each neutral [`ResourceKind`](crate::ResourceKind)
//! onto the real services a provider offers ([`Variant`]s), plus the region /
//! availability-zone topology used by failure analysis. Profiles are bundled
//! JSON snapshots — no network, no keys. AWS is the first (see `profiles/aws.json`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ResourceKind;

/// The bundled AWS profile, compiled into the binary.
const AWS_JSON: &str = include_str!("../../profiles/aws.json");

/// A concrete cloud provider's service catalogue and topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub provider: String,
    pub display_name: String,
    pub regions: Vec<Region>,
    /// Keyed by [`ResourceKind::slug`].
    pub variants: HashMap<String, Vec<Variant>>,
}

/// A region and the availability zones it contains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub azs: Vec<String>,
}

/// One concrete service a [`ResourceKind`](crate::ResourceKind) can be realised as.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub display_name: String,
    /// This variant is a single point of failure by construction (single-AZ,
    /// no built-in redundancy).
    #[serde(default)]
    pub spof: bool,
    /// If the variant survives a zone loss but with a brief interruption, how
    /// long that failover typically takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failover_seconds: Option<u32>,
}

impl ProviderProfile {
    /// The bundled AWS profile. Panics only if `profiles/aws.json` is malformed,
    /// which a unit test guards against.
    pub fn aws() -> Self {
        serde_json::from_str(AWS_JSON).expect("bundled profiles/aws.json is valid")
    }

    /// Look up a variant by kind and id.
    pub fn variant(&self, kind: ResourceKind, id: &str) -> Option<&Variant> {
        self.variants.get(kind.slug())?.iter().find(|v| v.id == id)
    }

    /// Look up a region by id.
    pub fn region(&self, id: &str) -> Option<&Region> {
        self.regions.iter().find(|r| r.id == id)
    }

    /// Whether `az` is a real zone of `region` in this profile.
    pub fn has_az(&self, region: &str, az: &str) -> bool {
        self.region(region)
            .is_some_and(|r| r.azs.iter().any(|a| a == az))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_aws_profile_parses() {
        let p = ProviderProfile::aws();
        assert_eq!(p.provider, "aws");
        assert!(p.region("us-east-1").is_some());
        assert!(p.has_az("us-east-1", "us-east-1a"));
        assert!(!p.has_az("us-east-1", "eu-west-1a"));
    }

    #[test]
    fn variant_lookup_reads_the_spof_flag() {
        let p = ProviderProfile::aws();
        assert!(
            p.variant(ResourceKind::Database, "rds-single-az")
                .unwrap()
                .spof
        );
        let multi = p.variant(ResourceKind::Database, "rds-multi-az").unwrap();
        assert!(!multi.spof);
        assert_eq!(multi.failover_seconds, Some(90));
        assert!(p.variant(ResourceKind::Database, "nope").is_none());
    }
}
