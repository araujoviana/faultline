#![forbid(unsafe_code)]

//! Pure compute core for **Faultline**, the cloud-architecture studio.
//! (The `strata-*` crate names are a legacy codename.)
//!
//! This crate is a plain `input -> output` library: an in-memory architecture
//! graph plus validation. No async, no I/O, no framework. It is the single
//! source of truth that both the UI and the WebMCP tools mutate (via the thin
//! `strata-wasm` binding layer).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub mod analysis;
pub mod explain;
pub mod iac;
pub mod lint;
pub mod profile;
pub mod propose;

/// A vendor-neutral cloud building block.
///
/// The first-pass provider (AWS / GCP) is deliberately not modelled yet; these
/// primitives map onto every major CSP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    Compute,
    Database,
    Queue,
    LoadBalancer,
    ObjectStore,
    Cache,
    Cdn,
    Dns,
    Functions,
    ApiGateway,
}

impl ResourceKind {
    /// Every kind, in catalog order.
    pub const ALL: [ResourceKind; 10] = [
        ResourceKind::Compute,
        ResourceKind::Database,
        ResourceKind::Queue,
        ResourceKind::LoadBalancer,
        ResourceKind::ObjectStore,
        ResourceKind::Cache,
        ResourceKind::Cdn,
        ResourceKind::Dns,
        ResourceKind::Functions,
        ResourceKind::ApiGateway,
    ];

    /// Stable lowercase identifier, also used as the id prefix.
    pub fn slug(&self) -> &'static str {
        match self {
            ResourceKind::Compute => "compute",
            ResourceKind::Database => "database",
            ResourceKind::Queue => "queue",
            ResourceKind::LoadBalancer => "load-balancer",
            ResourceKind::ObjectStore => "object-store",
            ResourceKind::Cache => "cache",
            ResourceKind::Cdn => "cdn",
            ResourceKind::Dns => "dns",
            ResourceKind::Functions => "functions",
            ResourceKind::ApiGateway => "api-gateway",
        }
    }

    /// A globally-distributed service with no single-region or single-AZ
    /// footprint: losing an availability zone or a region never takes it down
    /// directly (only losing everything it depends on does).
    pub fn is_global(&self) -> bool {
        matches!(self, ResourceKind::Cdn | ResourceKind::Dns)
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for ResourceKind {
    type Err = ArchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResourceKind::ALL
            .into_iter()
            .find(|k| k.slug() == s)
            .ok_or_else(|| ArchError::UnknownKind(s.to_string()))
    }
}

/// Where a resource lives in the provider's topology.
///
/// `az == Some` is a single zone (a zonal deployment); `az == None` with
/// `region == Some` is spread across the region's zones; both `None` means
/// unplaced (or a global service). The strings are validated against the active
/// [`profile::ProviderProfile`] at the binding layer, not here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub az: Option<String>,
}

/// A placed resource on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub kind: ResourceKind,
    pub label: String,
    pub x: f64,
    pub y: f64,
    /// Provider-specific service choice, e.g. `"rds-multi-az"`. `None` until the
    /// design is mapped onto a concrete provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default)]
    pub placement: Placement,
}

/// A directed dependency `from -> to` (e.g. load balancer -> compute).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

/// Everything that can go wrong when editing or validating an [`Architecture`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchError {
    UnknownKind(String),
    UnknownResource(String),
    SelfLoop(String),
    DuplicateEdge { from: String, to: String },
    DuplicateId(String),
    DanglingEdge { from: String, to: String },
    AzWithoutRegion(String),
}

impl fmt::Display for ArchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchError::UnknownKind(k) => write!(f, "unknown resource kind: {k}"),
            ArchError::UnknownResource(id) => write!(f, "no such resource: {id}"),
            ArchError::SelfLoop(id) => write!(f, "cannot connect {id} to itself"),
            ArchError::DuplicateEdge { from, to } => {
                write!(f, "{from} -> {to} is already connected")
            }
            ArchError::DuplicateId(id) => write!(f, "duplicate resource id: {id}"),
            ArchError::DanglingEdge { from, to } => {
                write!(f, "edge {from} -> {to} references a missing resource")
            }
            ArchError::AzWithoutRegion(id) => {
                write!(
                    f,
                    "cannot place {id} in an availability zone without a region"
                )
            }
        }
    }
}

impl std::error::Error for ArchError {}

/// An in-memory cloud-architecture graph.
///
/// `counters` tracks the next sequence number per kind so ids stay stable and
/// unique across removals. It is private but serialised, so a round-tripped
/// document keeps generating fresh ids.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Architecture {
    pub resources: Vec<Resource>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    counters: HashMap<String, u32>,
}

impl Architecture {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a resource with this id exists.
    pub fn contains(&self, id: &str) -> bool {
        self.resources.iter().any(|r| r.id == id)
    }

    /// Add a resource, returning its generated id (`"<kind>-<n>"`).
    pub fn add_resource(
        &mut self,
        kind: ResourceKind,
        label: impl Into<String>,
        x: f64,
        y: f64,
    ) -> String {
        let n = self.counters.entry(kind.slug().to_string()).or_insert(0);
        *n += 1;
        let id = format!("{}-{}", kind.slug(), n);
        self.resources.push(Resource {
            id: id.clone(),
            kind,
            label: label.into(),
            x,
            y,
            variant: None,
            placement: Placement::default(),
        });
        id
    }

    /// Set (or clear, with `None`) a resource's provider-specific variant.
    pub fn set_variant(&mut self, id: &str, variant: Option<String>) -> Result<(), ArchError> {
        let resource = self
            .resources
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| ArchError::UnknownResource(id.to_string()))?;
        resource.variant = variant;
        Ok(())
    }

    /// Set a resource's placement. `az` without `region` is rejected as nonsense.
    pub fn place(
        &mut self,
        id: &str,
        region: Option<String>,
        az: Option<String>,
    ) -> Result<(), ArchError> {
        if az.is_some() && region.is_none() {
            return Err(ArchError::AzWithoutRegion(id.to_string()));
        }
        let resource = self
            .resources
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| ArchError::UnknownResource(id.to_string()))?;
        resource.placement = Placement { region, az };
        Ok(())
    }

    /// Move a resource to a new canvas position.
    pub fn move_resource(&mut self, id: &str, x: f64, y: f64) -> Result<(), ArchError> {
        let resource = self
            .resources
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| ArchError::UnknownResource(id.to_string()))?;
        resource.x = x;
        resource.y = y;
        Ok(())
    }

    /// Connect `from -> to`. Rejects self-loops, unknown endpoints, and duplicates.
    pub fn connect(&mut self, from: &str, to: &str) -> Result<(), ArchError> {
        if from == to {
            return Err(ArchError::SelfLoop(from.to_string()));
        }
        if !self.contains(from) {
            return Err(ArchError::UnknownResource(from.to_string()));
        }
        if !self.contains(to) {
            return Err(ArchError::UnknownResource(to.to_string()));
        }
        if self.edges.iter().any(|e| e.from == from && e.to == to) {
            return Err(ArchError::DuplicateEdge {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
        });
        Ok(())
    }

    /// Remove a resource and any edges touching it.
    pub fn remove_resource(&mut self, id: &str) -> Result<(), ArchError> {
        if !self.contains(id) {
            return Err(ArchError::UnknownResource(id.to_string()));
        }
        self.resources.retain(|r| r.id != id);
        self.edges.retain(|e| e.from != id && e.to != id);
        Ok(())
    }

    /// Structural problems that shouldn't exist but might after a raw state load.
    pub fn validate(&self) -> Vec<ArchError> {
        let mut errors = Vec::new();

        let mut seen = HashSet::new();
        for r in &self.resources {
            if !seen.insert(r.id.as_str()) {
                errors.push(ArchError::DuplicateId(r.id.clone()));
            }
        }

        for e in &self.edges {
            if !self.contains(&e.from) || !self.contains(&e.to) {
                errors.push(ArchError::DanglingEdge {
                    from: e.from.clone(),
                    to: e.to.clone(),
                });
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_resource_generates_incrementing_ids_per_kind() {
        let mut a = Architecture::new();
        assert_eq!(
            a.add_resource(ResourceKind::Compute, "web", 0.0, 0.0),
            "compute-1"
        );
        assert_eq!(
            a.add_resource(ResourceKind::Compute, "worker", 0.0, 0.0),
            "compute-2"
        );
        assert_eq!(
            a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0),
            "database-1"
        );
        assert_eq!(a.resources.len(), 3);
    }

    #[test]
    fn connect_happy_path() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "lb", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        assert!(a.connect(&lb, &api).is_ok());
        assert_eq!(a.edges, vec![Edge { from: lb, to: api }]);
    }

    #[test]
    fn connect_unknown_endpoint_errors() {
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        assert_eq!(
            a.connect(&api, "ghost-1"),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
        assert_eq!(
            a.connect("ghost-1", &api),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
    }

    #[test]
    fn connect_self_loop_errors() {
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        assert_eq!(a.connect(&api, &api), Err(ArchError::SelfLoop(api)));
    }

    #[test]
    fn connect_duplicate_edge_errors() {
        let mut a = Architecture::new();
        let x = a.add_resource(ResourceKind::Compute, "a", 0.0, 0.0);
        let y = a.add_resource(ResourceKind::Cache, "b", 0.0, 0.0);
        a.connect(&x, &y).unwrap();
        assert_eq!(
            a.connect(&x, &y),
            Err(ArchError::DuplicateEdge { from: x, to: y })
        );
    }

    #[test]
    fn remove_resource_drops_incident_edges() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "lb", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        a.remove_resource(&api).unwrap();
        assert!(a.edges.is_empty());
        assert_eq!(a.resources.len(), 1);
    }

    #[test]
    fn remove_unknown_errors() {
        let mut a = Architecture::new();
        assert_eq!(
            a.remove_resource("ghost-1"),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
    }

    #[test]
    fn ids_stay_unique_after_removal() {
        let mut a = Architecture::new();
        let first = a.add_resource(ResourceKind::Compute, "a", 0.0, 0.0);
        a.remove_resource(&first).unwrap();
        let second = a.add_resource(ResourceKind::Compute, "b", 0.0, 0.0);
        assert_ne!(first, second);
        assert_eq!(second, "compute-2");
    }

    #[test]
    fn validate_flags_dangling_edge() {
        let a: Architecture = serde_json::from_str(
            r#"{"resources":[{"id":"compute-1","kind":"compute","label":"api","x":0,"y":0}],
                "edges":[{"from":"compute-1","to":"ghost-1"}]}"#,
        )
        .unwrap();
        assert_eq!(
            a.validate(),
            vec![ArchError::DanglingEdge {
                from: "compute-1".into(),
                to: "ghost-1".into()
            }]
        );
    }

    #[test]
    fn validate_clean_graph_has_no_errors() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "lb", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        assert!(a.validate().is_empty());
    }

    #[test]
    fn kind_from_str_roundtrips() {
        for k in ResourceKind::ALL {
            assert_eq!(k.to_string().parse::<ResourceKind>().unwrap(), k);
        }
        assert!("nope".parse::<ResourceKind>().is_err());
    }

    #[test]
    fn kind_serialises_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ResourceKind::LoadBalancer).unwrap(),
            "\"load-balancer\""
        );
        assert_eq!(
            serde_json::to_string(&ResourceKind::ApiGateway).unwrap(),
            "\"api-gateway\""
        );
    }

    #[test]
    fn only_cdn_and_dns_are_global() {
        let global: Vec<_> = ResourceKind::ALL
            .into_iter()
            .filter(ResourceKind::is_global)
            .collect();
        assert_eq!(global, [ResourceKind::Cdn, ResourceKind::Dns]);
    }

    #[test]
    fn architecture_json_roundtrip() {
        let mut a = Architecture::new();
        let x = a.add_resource(ResourceKind::LoadBalancer, "edge", 10.0, 20.0);
        let y = a.add_resource(ResourceKind::Compute, "api", 30.0, 40.0);
        a.connect(&x, &y).unwrap();
        let json = serde_json::to_string(&a).unwrap();
        let back: Architecture = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn set_variant_and_place_happy_path() {
        let mut a = Architecture::new();
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        a.set_variant(&db, Some("rds-multi-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), Some("us-east-1a".into()))
            .unwrap();
        let r = &a.resources[0];
        assert_eq!(r.variant.as_deref(), Some("rds-multi-az"));
        assert_eq!(r.placement.region.as_deref(), Some("us-east-1"));
        assert_eq!(r.placement.az.as_deref(), Some("us-east-1a"));

        a.set_variant(&db, None).unwrap();
        assert_eq!(a.resources[0].variant, None);
    }

    #[test]
    fn set_variant_and_place_reject_unknown_id() {
        let mut a = Architecture::new();
        assert_eq!(
            a.set_variant("ghost-1", Some("x".into())),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
        assert_eq!(
            a.place("ghost-1", Some("us-east-1".into()), None),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
    }

    #[test]
    fn move_resource_updates_position_and_rejects_unknown_id() {
        let mut a = Architecture::new();
        let n = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.move_resource(&n, 120.0, 240.0).unwrap();
        assert_eq!((a.resources[0].x, a.resources[0].y), (120.0, 240.0));
        assert_eq!(
            a.move_resource("ghost-1", 1.0, 1.0),
            Err(ArchError::UnknownResource("ghost-1".into()))
        );
    }

    #[test]
    fn place_rejects_az_without_region() {
        let mut a = Architecture::new();
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        assert_eq!(
            a.place(&db, None, Some("us-east-1a".into())),
            Err(ArchError::AzWithoutRegion(db))
        );
    }

    #[test]
    fn pre_session_json_without_variant_or_placement_still_loads() {
        // A document serialised before variants/placement existed.
        let a: Architecture = serde_json::from_str(
            r#"{"resources":[{"id":"compute-1","kind":"compute","label":"api","x":1,"y":2}],
                "edges":[]}"#,
        )
        .unwrap();
        assert_eq!(a.resources[0].variant, None);
        assert_eq!(a.resources[0].placement, Placement::default());
    }
}
