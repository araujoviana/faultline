//! Terraform HCL emitter: the architecture graph → a single `.tf` document.
//!
//! Pure `graph → string`, deterministic (resources id-sorted, edges
//! `(from,to)`-sorted, `depends_on` lists sorted + deduped), no new
//! dependencies. This is a one-way emit, not a parser.
//!
//! **Frozen scope boundary:** network (VPC, subnets, security groups) and IAM
//! are referenced as `var.*` input variables, never generated. Only the
//! `variable` blocks actually referenced are emitted.
//!
//! Edge convention (as everywhere in the core): `a -> b` means "**a depends on
//! b**"; a resource's out-edges are its dependencies.

use std::collections::BTreeSet;

use crate::profile::ProviderProfile;
use crate::{Architecture, Resource, ResourceKind};

/// Emit the whole architecture as one Terraform HCL document. Infallible: an
/// empty or all-unconfigured architecture yields the provider scaffold plus a
/// `# no resources` note.
pub fn emit_terraform(arch: &Architecture, profile: &ProviderProfile) -> String {
    let mut resources: Vec<&Resource> = arch.resources.iter().collect();
    resources.sort_by(|a, b| a.id.cmp(&b.id));

    let mut vars: BTreeSet<String> = BTreeSet::new();
    vars.insert("aws_region".to_string());

    let mut body = Hcl::new();

    for r in &resources {
        let Some(variant) = r.variant.as_deref() else {
            body.comment(&format!(
                "{} ({}) has no {} variant — run configure-resource",
                r.id, r.kind, profile.display_name
            ));
            body.blank();
            continue;
        };
        if profile.variant(r.kind, variant).is_none() {
            body.comment(&format!(
                "{} ({}) has an unrecognised variant \"{variant}\" — skipped",
                r.id, r.kind
            ));
            body.blank();
            continue;
        }
        emit_resource(&mut body, arch, r, variant, &mut vars);
        body.blank();
    }

    let mut out = Hcl::new();
    out.open("terraform");
    out.open("required_providers");
    out.open("aws =");
    out.attr("source", "\"hashicorp/aws\"");
    out.attr("version", "\"~> 5.0\"");
    out.close();
    out.close();
    out.close();
    out.blank();

    out.open("provider \"aws\"");
    out.attr("region", "var.aws_region");
    out.close();
    out.blank();

    for v in &vars {
        out.open(&format!("variable \"{v}\""));
        out.attr("type", var_type(v));
        out.close();
        out.blank();
    }

    let mut s = out.buf;
    if body.buf.trim().is_empty() {
        s.push_str("# no resources yet — add and configure resources, then regenerate\n");
    } else {
        s.push_str(&body.buf);
    }
    format!("{}\n", align(s.trim_end()))
}

/// Align consecutive `key = value` runs on the `=`, the way `terraform fmt`
/// does: a run is broken by any blank line, comment, block opener/closer, or a
/// change of indent.
fn align(src: &str) -> String {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let Some((indent0, _, _)) = split_attr(lines[i]) else {
            out.push(lines[i].to_string());
            i += 1;
            continue;
        };
        let mut run: Vec<(&str, &str, &str)> = Vec::new();
        while i < lines.len() {
            match split_attr(lines[i]) {
                Some((indent, key, val)) if indent == indent0 => {
                    run.push((indent, key, val));
                    i += 1;
                }
                _ => break,
            }
        }
        let width = run
            .iter()
            .map(|(_, k, _)| k.chars().count())
            .max()
            .unwrap_or(0);
        for (indent, key, val) in run {
            let pad = " ".repeat(width - key.chars().count());
            out.push(format!("{indent}{key}{pad} = {val}"));
        }
    }
    out.join("\n")
}

/// `(indent, key, value)` for an alignable `key = value` line whose value is
/// not a block opener; otherwise `None`.
fn split_attr(line: &str) -> Option<(&str, &str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return None;
    }
    let indent = &line[..line.len() - trimmed.len()];
    let (key, val) = trimmed.split_once(" = ")?;
    if val.ends_with('{') {
        return None;
    }
    let key_ok = (!key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_'))
        || (key.starts_with('"') && key.ends_with('"') && key.len() >= 2);
    key_ok.then_some((indent, key, val))
}

/// Terraform type for a generated input variable.
fn var_type(name: &str) -> &'static str {
    match name {
        "subnet_ids" | "security_group_ids" => "list(string)",
        _ => "string",
    }
}

/// `load-balancer-1` → `load_balancer_1` (a valid Terraform local name).
fn local_name(id: &str) -> String {
    id.replace('-', "_")
}

/// Escape a string for a double-quoted HCL literal.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// A minimal 2-space-indent HCL writer.
// ---------------------------------------------------------------------------

struct Hcl {
    buf: String,
    indent: usize,
}

impl Hcl {
    fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    fn pad(&mut self) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
    }

    fn line(&mut self, s: &str) {
        self.pad();
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn comment(&mut self, s: &str) {
        self.line(&format!("# {s}"));
    }

    /// Write `<header> {` and step in. `header` carries everything left of the
    /// brace, e.g. `resource "aws_lb" "edge"` or `tags =`.
    fn open(&mut self, header: &str) {
        self.line(&format!("{header} {{"));
        self.indent += 1;
    }

    fn close(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.line("}");
    }

    fn attr(&mut self, key: &str, value: &str) {
        self.line(&format!("{key} = {value}"));
    }

    fn blank(&mut self) {
        self.buf.push('\n');
    }

    /// The standard `tags` block: the diagram↔HCL correlation key plus the
    /// human-facing name.
    fn strata_tags(&mut self, r: &Resource) {
        self.open("tags =");
        self.attr("\"strata:id\"", &quote(&r.id));
        self.attr("Name", &quote(&r.label));
        self.close();
    }
}

// ---------------------------------------------------------------------------
// Dependency resolution
// ---------------------------------------------------------------------------

/// The Terraform address a `depends_on` entry for this resource points at.
fn primary_address(kind: ResourceKind, variant: &str, id: &str) -> Option<String> {
    let ln = local_name(id);
    let ty = match (kind, variant) {
        (ResourceKind::Compute, "ec2-asg") => "aws_autoscaling_group",
        (ResourceKind::Database, "rds-single-az" | "rds-multi-az") => "aws_db_instance",
        (ResourceKind::Database, "aurora") => "aws_rds_cluster",
        (ResourceKind::Database, "dynamodb") => "aws_dynamodb_table",
        (ResourceKind::LoadBalancer, "alb") => "aws_lb",
        (ResourceKind::ObjectStore, "s3") => "aws_s3_bucket",
        (ResourceKind::Cache, "elasticache") => "aws_elasticache_replication_group",
        (ResourceKind::Queue, "sqs") => "aws_sqs_queue",
        (ResourceKind::Cdn, "cloudfront") => "aws_cloudfront_distribution",
        (ResourceKind::Dns, "route53") => "aws_route53_record",
        (ResourceKind::Functions, "lambda") => "aws_lambda_function",
        (ResourceKind::ApiGateway, "apigw-http") => "aws_apigatewayv2_api",
        _ => return None,
    };
    Some(format!("{ty}.{ln}"))
}

/// The resources `id` depends on (its out-edges), each paired with its primary
/// Terraform address. Sorted by target id, deduped, unconfigured targets
/// dropped.
fn resolved_deps<'a>(arch: &'a Architecture, id: &str) -> Vec<(&'a Resource, String)> {
    let mut targets: Vec<&str> = arch
        .edges
        .iter()
        .filter(|e| e.from == id)
        .map(|e| e.to.as_str())
        .collect();
    targets.sort_unstable();
    targets.dedup();

    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for t in targets {
        let Some(dep) = arch.resources.iter().find(|r| r.id == t) else {
            continue;
        };
        let Some(v) = dep.variant.as_deref() else {
            continue;
        };
        if let Some(addr) = primary_address(dep.kind, v, &dep.id) {
            if seen.insert(addr.clone()) {
                out.push((dep, addr));
            }
        }
    }
    out
}

/// Write a `depends_on = [...]` line for every out-edge not already wired
/// through a concrete attribute reference (`wired`).
fn write_depends_on(h: &mut Hcl, arch: &Architecture, id: &str, wired: &BTreeSet<String>) {
    let mut deps: Vec<String> = resolved_deps(arch, id)
        .into_iter()
        .map(|(_, addr)| addr)
        .filter(|a| !wired.contains(a))
        .collect();
    deps.sort();
    deps.dedup();
    if !deps.is_empty() {
        h.attr("depends_on", &format!("[{}]", deps.join(", ")));
    }
}

/// Terraform addresses of a resource's dependencies that are ALB-fronted compute
/// (the `alb -> compute` edge is realised by the ASG's `target_group_arns`, so
/// the ALB needs no `depends_on` back to it).
fn alb_wired_compute(arch: &Architecture, id: &str) -> BTreeSet<String> {
    resolved_deps(arch, id)
        .into_iter()
        .filter(|(dep, _)| {
            dep.kind == ResourceKind::Compute && dep.variant.as_deref() == Some("ec2-asg")
        })
        .map(|(_, addr)| addr)
        .collect()
}

/// Inbound ALB target groups for a compute node (edges `alb -> compute`).
fn inbound_alb_target_groups(arch: &Architecture, compute_id: &str) -> Vec<String> {
    let mut froms: Vec<&str> = arch
        .edges
        .iter()
        .filter(|e| e.to == compute_id)
        .map(|e| e.from.as_str())
        .collect();
    froms.sort_unstable();
    froms.dedup();

    froms
        .into_iter()
        .filter_map(|f| arch.resources.iter().find(|r| r.id == f))
        .filter(|lb| lb.kind == ResourceKind::LoadBalancer && lb.variant.as_deref() == Some("alb"))
        .map(|lb| format!("aws_lb_target_group.{}.arn", local_name(&lb.id)))
        .collect()
}

// ---------------------------------------------------------------------------
// Per-kind emit
// ---------------------------------------------------------------------------

fn emit_resource(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    variant: &str,
    vars: &mut BTreeSet<String>,
) {
    let ln = local_name(&r.id);
    match (r.kind, variant) {
        (ResourceKind::Compute, "ec2-asg") => emit_ec2_asg(h, arch, r, &ln, vars),
        (ResourceKind::Database, "rds-single-az") => emit_rds(h, arch, r, &ln, false, vars),
        (ResourceKind::Database, "rds-multi-az") => emit_rds(h, arch, r, &ln, true, vars),
        (ResourceKind::Database, "aurora") => emit_aurora(h, arch, r, &ln, vars),
        (ResourceKind::Database, "dynamodb") => emit_dynamodb(h, arch, r, &ln),
        (ResourceKind::LoadBalancer, "alb") => emit_alb(h, arch, r, &ln, vars),
        (ResourceKind::ObjectStore, "s3") => emit_s3(h, arch, r, &ln, vars),
        (ResourceKind::Cache, "elasticache") => emit_elasticache(h, arch, r, &ln, vars),
        (ResourceKind::Queue, "sqs") => emit_sqs(h, arch, r, &ln, vars),
        (ResourceKind::Cdn, "cloudfront") => emit_cloudfront(h, arch, r, &ln, vars),
        (ResourceKind::Dns, "route53") => emit_route53(h, arch, r, &ln, vars),
        (ResourceKind::Functions, "lambda") => emit_lambda(h, arch, r, &ln, vars),
        (ResourceKind::ApiGateway, "apigw-http") => emit_apigw(h, arch, r, &ln),
        _ => h.comment(&format!(
            "{} ({}) variant \"{variant}\" is recognised but has no emitter yet",
            r.id, r.kind
        )),
    }
}

fn emit_ec2_asg(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    vars.insert("name_prefix".into());
    vars.insert("ami_id".into());
    vars.insert("security_group_ids".into());

    h.open(&format!("resource \"aws_launch_template\" \"{ln}\""));
    h.attr("name_prefix", &quote(&format!("{}-", r.id)));
    h.attr("image_id", "var.ami_id");
    h.attr("instance_type", "\"t3.small\"");
    h.attr("vpc_security_group_ids", "var.security_group_ids");
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_autoscaling_group\" \"{ln}\""));
    h.attr("name", &quote(&format!("${{var.name_prefix}}-{}", r.id)));
    h.attr("min_size", "2");
    h.attr("max_size", "6");
    h.attr("desired_capacity", "2");
    match (&r.placement.region, &r.placement.az) {
        (Some(_), Some(az)) => h.attr("availability_zones", &format!("[{}]", quote(az))),
        _ => {
            vars.insert("subnet_ids".into());
            h.attr("vpc_zone_identifier", "var.subnet_ids");
        }
    }
    let tgs = inbound_alb_target_groups(arch, &r.id);
    if !tgs.is_empty() {
        h.attr("target_group_arns", &format!("[{}]", tgs.join(", ")));
    }
    h.open("launch_template");
    h.attr("id", &format!("aws_launch_template.{ln}.id"));
    h.attr("version", "\"$Latest\"");
    h.close();
    h.open("tag");
    h.attr("key", "\"strata:id\"");
    h.attr("value", &quote(&r.id));
    h.attr("propagate_at_launch", "true");
    h.close();
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.close();
}

fn emit_rds(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    multi_az: bool,
    vars: &mut BTreeSet<String>,
) {
    vars.insert("name_prefix".into());
    vars.insert("security_group_ids".into());

    h.open(&format!("resource \"aws_db_instance\" \"{ln}\""));
    h.attr(
        "identifier",
        &quote(&format!("${{var.name_prefix}}-{}", r.id)),
    );
    h.attr("engine", "\"postgres\"");
    h.attr("instance_class", "\"db.t3.micro\"");
    h.attr("allocated_storage", "20");
    h.attr("multi_az", if multi_az { "true" } else { "false" });
    if !multi_az {
        if let Some(az) = &r.placement.az {
            h.attr("availability_zone", &quote(az));
        }
    }
    h.attr("vpc_security_group_ids", "var.security_group_ids");
    h.attr("manage_master_user_password", "true");
    h.attr("username", "\"app\"");
    h.attr("skip_final_snapshot", "true");
    h.comment("db_subnet_group_name: attach to your VPC's DB subnet group");
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_aurora(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    vars.insert("name_prefix".into());
    vars.insert("security_group_ids".into());

    h.open(&format!("resource \"aws_rds_cluster\" \"{ln}\""));
    h.attr(
        "cluster_identifier",
        &quote(&format!("${{var.name_prefix}}-{}", r.id)),
    );
    h.attr("engine", "\"aurora-postgresql\"");
    h.attr("vpc_security_group_ids", "var.security_group_ids");
    h.attr("manage_master_user_password", "true");
    h.attr("master_username", "\"app\"");
    h.attr("skip_final_snapshot", "true");
    h.comment("db_subnet_group_name: attach to your VPC's DB subnet group");
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_rds_cluster_instance\" \"{ln}\""));
    h.attr("count", "2");
    h.attr(
        "identifier",
        &quote(&format!("${{var.name_prefix}}-{}-${{count.index}}", r.id)),
    );
    h.attr("cluster_identifier", &format!("aws_rds_cluster.{ln}.id"));
    h.attr("engine", "\"aurora-postgresql\"");
    h.attr("instance_class", "\"db.t3.medium\"");
    h.close();
}

fn emit_dynamodb(h: &mut Hcl, arch: &Architecture, r: &Resource, ln: &str) {
    h.open(&format!("resource \"aws_dynamodb_table\" \"{ln}\""));
    h.attr("name", &quote(&r.id));
    h.attr("billing_mode", "\"PAY_PER_REQUEST\"");
    h.attr("hash_key", "\"id\"");
    h.open("attribute");
    h.attr("name", "\"id\"");
    h.attr("type", "\"S\"");
    h.close();
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_alb(h: &mut Hcl, arch: &Architecture, r: &Resource, ln: &str, vars: &mut BTreeSet<String>) {
    vars.insert("name_prefix".into());
    vars.insert("vpc_id".into());
    vars.insert("subnet_ids".into());
    vars.insert("security_group_ids".into());

    h.open(&format!("resource \"aws_lb\" \"{ln}\""));
    h.attr("name", &quote(&format!("${{var.name_prefix}}-{}", r.id)));
    h.attr("load_balancer_type", "\"application\"");
    h.attr("subnets", "var.subnet_ids");
    h.attr("security_groups", "var.security_group_ids");
    write_depends_on(h, arch, &r.id, &alb_wired_compute(arch, &r.id));
    h.strata_tags(r);
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_lb_target_group\" \"{ln}\""));
    h.attr("name", &quote(&format!("${{var.name_prefix}}-{}", r.id)));
    h.attr("port", "80");
    h.attr("protocol", "\"HTTP\"");
    h.attr("vpc_id", "var.vpc_id");
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_lb_listener\" \"{ln}\""));
    h.attr("load_balancer_arn", &format!("aws_lb.{ln}.arn"));
    h.attr("port", "80");
    h.attr("protocol", "\"HTTP\"");
    h.open("default_action");
    h.attr("type", "\"forward\"");
    h.attr("target_group_arn", &format!("aws_lb_target_group.{ln}.arn"));
    h.close();
    h.close();
}

fn emit_s3(h: &mut Hcl, arch: &Architecture, r: &Resource, ln: &str, vars: &mut BTreeSet<String>) {
    vars.insert("name_prefix".into());
    h.open(&format!("resource \"aws_s3_bucket\" \"{ln}\""));
    h.attr("bucket", &quote(&format!("${{var.name_prefix}}-{}", r.id)));
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_elasticache(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    vars.insert("name_prefix".into());
    vars.insert("security_group_ids".into());

    h.open(&format!(
        "resource \"aws_elasticache_replication_group\" \"{ln}\""
    ));
    h.attr(
        "replication_group_id",
        &quote(&format!("${{var.name_prefix}}-{}", r.id)),
    );
    h.attr("description", &quote(&r.label));
    h.attr("engine", "\"redis\"");
    h.attr("node_type", "\"cache.t3.micro\"");
    h.attr("num_cache_clusters", "2");
    h.attr("automatic_failover_enabled", "true");
    h.attr("multi_az_enabled", "true");
    h.attr("security_group_ids", "var.security_group_ids");
    h.comment("subnet_group_name: attach to your VPC's cache subnet group");
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_sqs(h: &mut Hcl, arch: &Architecture, r: &Resource, ln: &str, vars: &mut BTreeSet<String>) {
    vars.insert("name_prefix".into());
    h.open(&format!("resource \"aws_sqs_queue\" \"{ln}\""));
    h.attr("name", &quote(&format!("${{var.name_prefix}}-{}", r.id)));
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_cloudfront(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    let deps = resolved_deps(arch, &r.id);
    let mut wired = BTreeSet::new();

    // Origin: prefer a single ALB / S3 dependency, else an input variable.
    let (origin_domain, origin_addr) = deps
        .iter()
        .find_map(|(dep, addr)| match (dep.kind, dep.variant.as_deref()) {
            (ResourceKind::LoadBalancer, Some("alb")) => Some((
                format!("aws_lb.{}.dns_name", local_name(&dep.id)),
                Some(addr.clone()),
            )),
            (ResourceKind::ObjectStore, Some("s3")) => Some((
                format!(
                    "aws_s3_bucket.{}.bucket_regional_domain_name",
                    local_name(&dep.id)
                ),
                Some(addr.clone()),
            )),
            _ => None,
        })
        .unwrap_or_else(|| {
            let v = format!("{ln}_origin_domain");
            vars.insert(v.clone());
            (format!("var.{v}"), None)
        });
    if let Some(a) = origin_addr {
        wired.insert(a);
    }

    h.open(&format!(
        "resource \"aws_cloudfront_distribution\" \"{ln}\""
    ));
    h.attr("enabled", "true");
    h.attr("is_ipv6_enabled", "true");
    h.attr("default_root_object", "\"index.html\"");
    h.open("origin");
    h.attr("domain_name", &origin_domain);
    h.attr("origin_id", &quote(&format!("{}-origin", r.id)));
    h.open("custom_origin_config");
    h.attr("http_port", "80");
    h.attr("https_port", "443");
    h.attr("origin_protocol_policy", "\"https-only\"");
    h.attr("origin_ssl_protocols", "[\"TLSv1.2\"]");
    h.close();
    h.close();
    h.open("default_cache_behavior");
    h.attr("target_origin_id", &quote(&format!("{}-origin", r.id)));
    h.attr("viewer_protocol_policy", "\"redirect-to-https\"");
    h.attr("allowed_methods", "[\"GET\", \"HEAD\", \"OPTIONS\"]");
    h.attr("cached_methods", "[\"GET\", \"HEAD\"]");
    h.open("forwarded_values");
    h.attr("query_string", "true");
    h.open("cookies");
    h.attr("forward", "\"none\"");
    h.close();
    h.close();
    h.close();
    h.open("restrictions");
    h.open("geo_restriction");
    h.attr("restriction_type", "\"none\"");
    h.close();
    h.close();
    h.open("viewer_certificate");
    h.attr("cloudfront_default_certificate", "true");
    h.close();
    write_depends_on(h, arch, &r.id, &wired);
    h.strata_tags(r);
    h.close();
}

fn emit_route53(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    let zone_var = format!("{ln}_zone_name");
    vars.insert(zone_var.clone());

    let deps = resolved_deps(arch, &r.id);
    let mut wired = BTreeSet::new();
    let alias = deps.iter().find_map(|(dep, addr)| {
        let d = local_name(&dep.id);
        match (dep.kind, dep.variant.as_deref()) {
            (ResourceKind::Cdn, Some("cloudfront")) => Some((
                format!("aws_cloudfront_distribution.{d}.domain_name"),
                format!("aws_cloudfront_distribution.{d}.hosted_zone_id"),
                addr.clone(),
            )),
            (ResourceKind::LoadBalancer, Some("alb")) => Some((
                format!("aws_lb.{d}.dns_name"),
                format!("aws_lb.{d}.zone_id"),
                addr.clone(),
            )),
            _ => None,
        }
    });

    h.open(&format!("resource \"aws_route53_zone\" \"{ln}\""));
    h.attr("name", &format!("var.{zone_var}"));
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_route53_record\" \"{ln}\""));
    h.attr("zone_id", &format!("aws_route53_zone.{ln}.zone_id"));
    h.attr("name", &format!("var.{zone_var}"));
    h.attr("type", "\"A\"");
    match alias {
        Some((name, zone_id, addr)) => {
            wired.insert(addr);
            h.open("alias");
            h.attr("name", &name);
            h.attr("zone_id", &zone_id);
            h.attr("evaluate_target_health", "true");
            h.close();
        }
        None => {
            let target_var = format!("{ln}_target");
            vars.insert(target_var.clone());
            h.attr("ttl", "300");
            h.attr("records", &format!("[var.{target_var}]"));
        }
    }
    write_depends_on(h, arch, &r.id, &wired);
    h.close();
}

fn emit_lambda(
    h: &mut Hcl,
    arch: &Architecture,
    r: &Resource,
    ln: &str,
    vars: &mut BTreeSet<String>,
) {
    vars.insert("name_prefix".into());
    let pkg = format!("{ln}_package");
    let role = format!("{ln}_role_arn");
    vars.insert(pkg.clone());
    vars.insert(role.clone());

    h.open(&format!("resource \"aws_lambda_function\" \"{ln}\""));
    h.attr(
        "function_name",
        &quote(&format!("${{var.name_prefix}}-{}", r.id)),
    );
    h.attr("runtime", "\"nodejs20.x\"");
    h.attr("handler", "\"index.handler\"");
    h.attr("filename", &format!("var.{pkg}"));
    h.attr("role", &format!("var.{role}"));
    h.comment("role: an IAM role ARN — IAM is not modelled by the studio");
    h.attr("timeout", "30");
    h.attr("memory_size", "256");
    write_depends_on(h, arch, &r.id, &BTreeSet::new());
    h.strata_tags(r);
    h.close();
}

fn emit_apigw(h: &mut Hcl, arch: &Architecture, r: &Resource, ln: &str) {
    h.open(&format!("resource \"aws_apigatewayv2_api\" \"{ln}\""));
    h.attr("name", &quote(&r.id));
    h.attr("protocol_type", "\"HTTP\"");
    h.close();
    h.blank();

    h.open(&format!("resource \"aws_apigatewayv2_stage\" \"{ln}\""));
    h.attr("api_id", &format!("aws_apigatewayv2_api.{ln}.id"));
    h.attr("name", "\"$default\"");
    h.attr("auto_deploy", "true");
    h.close();

    // Wire each Lambda dependency as a proxy integration + route.
    for (dep, _) in resolved_deps(arch, &r.id) {
        if dep.kind != ResourceKind::Functions || dep.variant.as_deref() != Some("lambda") {
            continue;
        }
        let dl = local_name(&dep.id);
        let pair = format!("{ln}_{dl}");
        h.blank();
        h.open(&format!(
            "resource \"aws_apigatewayv2_integration\" \"{pair}\""
        ));
        h.attr("api_id", &format!("aws_apigatewayv2_api.{ln}.id"));
        h.attr("integration_type", "\"AWS_PROXY\"");
        h.attr(
            "integration_uri",
            &format!("aws_lambda_function.{dl}.invoke_arn"),
        );
        h.attr("payload_format_version", "\"2.0\"");
        h.close();
        h.blank();
        h.open(&format!("resource \"aws_apigatewayv2_route\" \"{pair}\""));
        h.attr("api_id", &format!("aws_apigatewayv2_api.{ln}.id"));
        h.attr("route_key", "\"ANY /{proxy+}\"");
        h.attr(
            "target",
            &format!("\"integrations/${{aws_apigatewayv2_integration.{pair}.id}}\""),
        );
        h.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceKind;

    fn aws() -> ProviderProfile {
        ProviderProfile::aws()
    }

    /// Collapse whitespace so substring checks ignore `terraform fmt` alignment.
    fn norm(s: &str) -> String {
        s.lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn one(kind: ResourceKind, variant: &str) -> Architecture {
        let mut a = Architecture::new();
        let id = a.add_resource(kind, "thing", 0.0, 0.0);
        a.set_variant(&id, Some(variant.into())).unwrap();
        a
    }

    #[test]
    fn empty_arch_emits_scaffold_only() {
        let out = emit_terraform(&Architecture::new(), &aws());
        assert!(norm(&out).contains("terraform {"));
        assert!(norm(&out).contains("provider \"aws\""));
        assert!(norm(&out).contains("# no resources yet"));
        assert!(!norm(&out).contains("resource \""));
    }

    #[test]
    fn unconfigured_resource_emits_comment_and_no_block() {
        let mut a = Architecture::new();
        a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("# compute-1 (compute) has no Amazon Web Services variant"));
        assert!(!norm(&out).contains("resource \"aws_"));
    }

    #[test]
    fn s3_bucket_snapshot() {
        let out = emit_terraform(&one(ResourceKind::ObjectStore, "s3"), &aws());
        let expected = r#"terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

variable "aws_region" {
  type = string
}

variable "name_prefix" {
  type = string
}

resource "aws_s3_bucket" "object_store_1" {
  bucket = "${var.name_prefix}-object-store-1"
  tags = {
    "strata:id" = "object-store-1"
    Name        = "thing"
  }
}
"#;
        assert_eq!(out, expected);
    }

    #[test]
    fn rds_single_az_sets_multi_az_false_and_zone() {
        let mut a = one(ResourceKind::Database, "rds-single-az");
        a.place(
            "database-1",
            Some("us-east-1".into()),
            Some("us-east-1a".into()),
        )
        .unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("multi_az = false"));
        assert!(norm(&out).contains("availability_zone = \"us-east-1a\""));
    }

    #[test]
    fn rds_multi_az_flips_flag_and_drops_zone() {
        let mut a = one(ResourceKind::Database, "rds-multi-az");
        a.place("database-1", Some("us-east-1".into()), None)
            .unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("multi_az = true"));
        assert!(!norm(&out).contains("availability_zone ="));
    }

    #[test]
    fn aurora_emits_cluster_and_instance() {
        let out = emit_terraform(&one(ResourceKind::Database, "aurora"), &aws());
        assert!(norm(&out).contains("resource \"aws_rds_cluster\" \"database_1\""));
        assert!(norm(&out).contains("resource \"aws_rds_cluster_instance\" \"database_1\""));
        assert!(norm(&out).contains("count = 2"));
    }

    #[test]
    fn dynamodb_table() {
        let out = emit_terraform(&one(ResourceKind::Database, "dynamodb"), &aws());
        assert!(norm(&out).contains("resource \"aws_dynamodb_table\" \"database_1\""));
        assert!(norm(&out).contains("billing_mode = \"PAY_PER_REQUEST\""));
    }

    #[test]
    fn ec2_asg_emits_launch_template_and_group() {
        let out = emit_terraform(&one(ResourceKind::Compute, "ec2-asg"), &aws());
        assert!(norm(&out).contains("resource \"aws_launch_template\" \"compute_1\""));
        assert!(norm(&out).contains("resource \"aws_autoscaling_group\" \"compute_1\""));
        assert!(norm(&out).contains("vpc_zone_identifier = var.subnet_ids"));
    }

    #[test]
    fn zonal_asg_uses_availability_zones() {
        let mut a = one(ResourceKind::Compute, "ec2-asg");
        a.place(
            "compute-1",
            Some("us-east-1".into()),
            Some("us-east-1b".into()),
        )
        .unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("availability_zones = [\"us-east-1b\"]"));
        assert!(!norm(&out).contains("vpc_zone_identifier"));
    }

    #[test]
    fn alb_emits_lb_listener_and_target_group() {
        let out = emit_terraform(&one(ResourceKind::LoadBalancer, "alb"), &aws());
        assert!(norm(&out).contains("resource \"aws_lb\" \"load_balancer_1\""));
        assert!(norm(&out).contains("resource \"aws_lb_listener\" \"load_balancer_1\""));
        assert!(norm(&out).contains("resource \"aws_lb_target_group\" \"load_balancer_1\""));
    }

    /// LB -> compute wires the ASG to the LB's target group.
    #[test]
    fn alb_to_compute_edge_wires_target_group_arns() {
        let mut a = Architecture::new();
        let lb = a.add_resource(ResourceKind::LoadBalancer, "edge", 0.0, 0.0);
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        a.connect(&lb, &api).unwrap();
        a.set_variant(&lb, Some("alb".into())).unwrap();
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(
            norm(&out).contains("target_group_arns = [aws_lb_target_group.load_balancer_1.arn]")
        );
        // The LB depends on compute, but that edge is wired via the ASG, so no
        // depends_on on the LB pointing back at compute.
        assert!(!norm(&out).contains("depends_on = [aws_autoscaling_group.compute_1]"));
    }

    #[test]
    fn other_edges_emit_sorted_depends_on() {
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::Compute, "api", 0.0, 0.0);
        let db = a.add_resource(ResourceKind::Database, "orders", 0.0, 0.0);
        let cache = a.add_resource(ResourceKind::Cache, "sess", 0.0, 0.0);
        a.connect(&api, &db).unwrap();
        a.connect(&api, &cache).unwrap();
        a.set_variant(&api, Some("ec2-asg".into())).unwrap();
        a.set_variant(&db, Some("rds-multi-az".into())).unwrap();
        a.set_variant(&cache, Some("elasticache".into())).unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains(
            "depends_on = [aws_db_instance.database_1, aws_elasticache_replication_group.cache_1]"
        ));
    }

    #[test]
    fn cloudfront_takes_its_origin_from_an_alb_dependency() {
        let mut a = Architecture::new();
        let cdn = a.add_resource(ResourceKind::Cdn, "edge", 0.0, 0.0);
        let lb = a.add_resource(ResourceKind::LoadBalancer, "alb", 0.0, 0.0);
        a.connect(&cdn, &lb).unwrap();
        a.set_variant(&cdn, Some("cloudfront".into())).unwrap();
        a.set_variant(&lb, Some("alb".into())).unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("domain_name = aws_lb.load_balancer_1.dns_name"));
        assert!(!norm(&out).contains("origin_domain"));
    }

    #[test]
    fn route53_aliases_to_a_cloudfront_dependency() {
        let mut a = Architecture::new();
        let dns = a.add_resource(ResourceKind::Dns, "zone", 0.0, 0.0);
        let cdn = a.add_resource(ResourceKind::Cdn, "edge", 0.0, 0.0);
        a.connect(&dns, &cdn).unwrap();
        a.set_variant(&dns, Some("route53".into())).unwrap();
        a.set_variant(&cdn, Some("cloudfront".into())).unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(norm(&out).contains("alias {"));
        assert!(norm(&out).contains("name = aws_cloudfront_distribution.cdn_1.domain_name"));
    }

    #[test]
    fn apigw_wires_a_lambda_integration_and_route() {
        let mut a = Architecture::new();
        let api = a.add_resource(ResourceKind::ApiGateway, "gw", 0.0, 0.0);
        let f = a.add_resource(ResourceKind::Functions, "handler", 0.0, 0.0);
        a.connect(&api, &f).unwrap();
        a.set_variant(&api, Some("apigw-http".into())).unwrap();
        a.set_variant(&f, Some("lambda".into())).unwrap();
        let out = emit_terraform(&a, &aws());
        assert!(
            out.contains("resource \"aws_apigatewayv2_integration\" \"api_gateway_1_functions_1\"")
        );
        assert!(norm(&out).contains("integration_uri = aws_lambda_function.functions_1.invoke_arn"));
        assert!(norm(&out)
            .contains("resource \"aws_apigatewayv2_route\" \"api_gateway_1_functions_1\""));
    }

    #[test]
    fn sqs_queue() {
        let out = emit_terraform(&one(ResourceKind::Queue, "sqs"), &aws());
        assert!(norm(&out).contains("resource \"aws_sqs_queue\" \"queue_1\""));
    }

    #[test]
    fn only_referenced_variables_are_emitted() {
        let out = emit_terraform(&one(ResourceKind::ObjectStore, "s3"), &aws());
        assert!(norm(&out).contains("variable \"name_prefix\""));
        assert!(norm(&out).contains("variable \"aws_region\""));
        assert!(!norm(&out).contains("variable \"ami_id\""));
        assert!(!norm(&out).contains("variable \"vpc_id\""));
    }

    #[test]
    fn output_is_byte_stable_across_calls() {
        let a = demo_arch();
        assert_eq!(emit_terraform(&a, &aws()), emit_terraform(&a, &aws()));
    }

    #[test]
    fn resources_emit_in_id_order() {
        let a = demo_arch();
        let out = emit_terraform(&a, &aws());
        let db = out.find("\"database_1\"").unwrap();
        let lb = out.find("\"load_balancer_1\"").unwrap();
        assert!(db < lb, "database-1 block should precede load-balancer-1");
    }

    #[test]
    fn local_names_sanitize_hyphens() {
        let out = emit_terraform(&one(ResourceKind::LoadBalancer, "alb"), &aws());
        assert!(norm(&out).contains("\"aws_lb\" \"load_balancer_1\""));
    }

    /// alb -> compute -> rds-multi-az, all regional in us-east-1.
    fn demo_arch() -> Architecture {
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
        a.set_variant(&db, Some("rds-multi-az".into())).unwrap();
        a.place(&db, Some("us-east-1".into()), None).unwrap();
        a
    }

    #[test]
    fn full_demo_arch_contains_the_key_wires() {
        let out = emit_terraform(&demo_arch(), &aws());
        assert!(norm(&out).contains("resource \"aws_autoscaling_group\" \"compute_1\""));
        assert!(norm(&out).contains("resource \"aws_db_instance\" \"database_1\""));
        assert!(norm(&out).contains("multi_az = true"));
        assert!(
            norm(&out).contains("target_group_arns = [aws_lb_target_group.load_balancer_1.arn]")
        );
        assert!(norm(&out).contains("depends_on = [aws_db_instance.database_1]"));
        assert!(norm(&out).contains("variable \"vpc_id\""));
    }
}
