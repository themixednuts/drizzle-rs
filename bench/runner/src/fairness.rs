//! Two-axis fairness: identical *within* a family, declared *across* families.
//!
//! These are two different meanings of the word and blurring them produces a
//! table that looks comparable and is not:
//!
//! - **Within a database family** every target must declare the *same* harness —
//!   same worker count, same pool size, same engine tuning. Only then is a
//!   difference in the numbers attributable to the library rather than to the
//!   configuration it was handed. Drift is a hard error, not a warning: a
//!   silently unequal library comparison is worse than no comparison, and a
//!   warning buried in the manifest is silent in every way that matters.
//! - **Across families** nothing is constrained. PostgreSQL over TCP with a
//!   pool of 8 and an embedded SQLite with a pool of 1 *should* differ; the
//!   difference is the stack comparison. What matters is that the configuration
//!   is recorded per family so no reader mistakes it for a library difference,
//!   which is what [`harness`] writes into the manifest.
//!
//! A "family" is a **comparison group**: the set of targets claiming to be
//! directly comparable. It usually maps onto the database engine, but it splits
//! where the harness cannot honestly be equalised. `sqlite-ts` is separate from
//! `sqlite` because `bun:sqlite` is a synchronous API on a single-threaded
//! runtime — a pool of 8 there is theatre, and forcing one to match the Rust
//! targets would cripple it in the name of fairness. drizzle-rs on rusqlite
//! versus drizzle-orm on Bun differs in language, runtime and concurrency model,
//! which makes it a *stack* comparison and therefore the across-family axis;
//! inside `sqlite-ts`, drizzle-orm versus bun:sqlite is a real library
//! comparison. Splitting a group changes enforcement and delta scoping only —
//! both groups still appear in one table.
//!
//! Families come from `fair.family`, which each target declares. It is not
//! inferred: `db.profile` separates configurations *inside* a group (prepared
//! vs unprepared) and `fair.db` names the SQL dialect shared by several engines,
//! so neither identifies the bracket a target competes in. It is also not taken
//! from the spec file a target arrived in — publish-class runs already execute
//! several PostgreSQL spec files back to back inside one job.

use crate::code::{Code, Fail};
use crate::model::{HarnessDoc, Target};
use std::collections::BTreeMap;

/// Enforce within-family harness identity and describe each family's harness.
///
/// # Errors
///
/// Fails when two targets in the same family declare different `fair.workers`,
/// `fair.pool`, or `fair.tuning`. The message names both targets and the field,
/// because the fix is always to change one of them.
pub fn harness(targets: &[Target]) -> Result<Vec<HarnessDoc>, Fail> {
    let mut families: BTreeMap<&str, Vec<&Target>> = BTreeMap::new();
    for target in targets {
        families
            .entry(target.fair.family.as_str())
            .or_default()
            .push(target);
    }

    families
        .into_iter()
        .map(|(family, members)| family_harness(family, &members))
        .collect()
}

fn family_harness(family: &str, members: &[&Target]) -> Result<HarnessDoc, Fail> {
    let (exempt, compared): (Vec<&Target>, Vec<&Target>) =
        members.iter().partition(|target| is_exempt(target));

    let Some(reference) = compared.first() else {
        // Every member opted out of the comparison, so there is no harness to
        // enforce and none is claimed.
        return Ok(HarnessDoc {
            family: family.to_string(),
            targets: Vec::new(),
            workers: None,
            pool: None,
            tuning: None,
            within_family_identical: false,
            exempt: ids(&exempt),
        });
    };

    for target in compared.iter().skip(1) {
        check(family, reference, target, "workers", |t| {
            t.fair.workers.to_string()
        })?;
        check(family, reference, target, "pool", |t| {
            t.fair.pool.to_string()
        })?;
        check(family, reference, target, "tuning", |t| {
            t.fair.tuning.clone()
        })?;
    }

    Ok(HarnessDoc {
        family: family.to_string(),
        targets: ids(&compared),
        workers: Some(reference.fair.workers),
        pool: Some(reference.fair.pool),
        tuning: Some(reference.fair.tuning.clone()),
        within_family_identical: true,
        exempt: ids(&exempt),
    })
}

fn check(
    family: &str,
    reference: &Target,
    target: &Target,
    field: &str,
    value: impl Fn(&Target) -> String,
) -> Result<(), Fail> {
    let (found, expected) = (value(target), value(reference));
    if found == expected {
        return Ok(());
    }
    Err(Fail::new(
        Code::InvalidInput,
        format!(
            "unfair {family} comparison: target {} declares fair.{field}={found} \
             but target {} declares fair.{field}={expected}. Targets in the same \
             database family must run an identical harness, otherwise the ranking \
             compares configurations instead of libraries. Change one of them, or \
             move it to its own family.",
            target.id, reference.id
        ),
    ))
}

/// A target serving from a replicated in-process cache has no connection pool
/// to equalise, so comparing its pool size against a SQL client's is meaningless.
/// It is listed in the manifest rather than dropped.
fn is_exempt(target: &Target) -> bool {
    matches!(target.data_access.as_deref(), Some("in-process-cache"))
}

fn ids(targets: &[&Target]) -> Vec<String> {
    targets.iter().map(|target| target.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Contract, Db, DisplayMeta, Driver, Exec, Fair, NameVer, Pool, Proc, Target, Wire,
    };

    fn target(id: &str, family: &str, workers: u32, pool: u32, tuning: &str) -> Target {
        Target {
            version: "v1".to_string(),
            id: id.to_string(),
            display: DisplayMeta {
                name: id.to_string(),
                description: None,
            },
            lang: "rust".to_string(),
            group: None,
            data_access: None,
            sql_variant: None,
            runtime: NameVer {
                name: "rust".to_string(),
                ver: "1.95.0".to_string(),
            },
            orm: NameVer {
                name: "drizzle-rs".to_string(),
                ver: "0.1.15".to_string(),
            },
            driver: Driver {
                name: "rusqlite".to_string(),
                ver: "0.39.0".to_string(),
                transport: None,
            },
            proc: Proc {
                mode: "single".to_string(),
                workers,
            },
            pool: Pool {
                max: pool,
                min: None,
                acquire_ms: None,
            },
            db: Db {
                profile: "sqlite".to_string(),
                hash: "sha256:0".to_string(),
                prepared: None,
            },
            wire: Wire {
                format: "json".to_string(),
            },
            fair: Fair {
                family: family.to_string(),
                workers,
                pool,
                db: "sqlite".to_string(),
                schema: "sha256:0".to_string(),
                contract: "v1".to_string(),
                tuning: tuning.to_string(),
            },
            contract: Contract {
                ver: "v1".to_string(),
            },
            parity: exec(),
            warmup: None,
            load: exec(),
            server: None,
        }
    }

    fn exec() -> Exec {
        Exec {
            cmd: vec!["true".to_string()],
            cwd: None,
            env: Default::default(),
            timeout_s: None,
        }
    }

    #[test]
    fn a_family_running_one_harness_is_recorded_as_verified() {
        let targets = [
            target("drizzle-rs-sqlite", "sqlite", 1, 8, "WAL"),
            target("rusqlite-prepared", "sqlite", 1, 8, "WAL"),
        ];
        let blocks = harness(&targets).expect("identical harness");

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].family, "sqlite");
        assert_eq!(blocks[0].workers, Some(1));
        assert_eq!(blocks[0].pool, Some(8));
        assert_eq!(blocks[0].tuning.as_deref(), Some("WAL"));
        assert!(blocks[0].within_family_identical);
        assert_eq!(
            blocks[0].targets,
            ["drizzle-rs-sqlite", "rusqlite-prepared"]
        );
    }

    #[test]
    fn pool_drift_inside_a_family_fails_the_run() {
        let targets = [
            target("drizzle-rs-sqlite", "sqlite", 1, 8, "WAL"),
            target("bun-sqlite", "sqlite", 1, 1, "WAL"),
        ];
        let err = harness(&targets).expect_err("pool drift must fail");

        assert_eq!(err.code, Code::InvalidInput);
        assert!(err.msg.contains("unfair sqlite comparison"), "{}", err.msg);
        assert!(err.msg.contains("bun-sqlite"), "{}", err.msg);
        assert!(err.msg.contains("drizzle-rs-sqlite"), "{}", err.msg);
        assert!(err.msg.contains("fair.pool=1"), "{}", err.msg);
        assert!(err.msg.contains("fair.pool=8"), "{}", err.msg);
    }

    #[test]
    fn worker_and_tuning_drift_fail_too() {
        let workers = [
            target("a", "postgres", 1, 8, "stock"),
            target("b", "postgres", 4, 8, "stock"),
        ];
        let err = harness(&workers).expect_err("worker drift");
        assert!(err.msg.contains("fair.workers=4"), "{}", err.msg);

        let tuning = [
            target("a", "postgres", 1, 8, "stock postgres:18-alpine"),
            target("b", "postgres", 1, 8, "shared_buffers=4GB"),
        ];
        let err = harness(&tuning).expect_err("tuning drift");
        assert!(
            err.msg.contains("fair.tuning=shared_buffers=4GB"),
            "{}",
            err.msg
        );
    }

    #[test]
    fn different_families_may_differ_freely() {
        let targets = [
            target("drizzle-rs-sqlite", "sqlite", 1, 8, "WAL"),
            target("drizzle-rs-turso", "turso", 1, 4, "WAL, turso"),
            target(
                "drizzle-rs-pg",
                "postgres",
                1,
                8,
                "stock postgres:18-alpine",
            ),
        ];
        let blocks = harness(&targets).expect("cross-family differences are allowed");

        assert_eq!(blocks.len(), 3);
        // Sorted by family so the manifest is stable across runs.
        assert_eq!(
            blocks.iter().map(|b| b.family.as_str()).collect::<Vec<_>>(),
            ["postgres", "sqlite", "turso"]
        );
        assert_eq!(blocks[2].pool, Some(4));
        assert!(blocks.iter().all(|b| b.within_family_identical));
    }

    #[test]
    fn an_in_process_cache_is_listed_as_exempt_not_dropped() {
        let mut cache = target("spacetime-sdk-rs", "spacetimedb", 1, 1, "cache");
        cache.data_access = Some("in-process-cache".to_string());
        let targets = [
            target("spacetime-pgwire-rs", "spacetimedb", 1, 4, "stock"),
            cache,
        ];
        let blocks = harness(&targets).expect("exempt targets do not force drift");

        assert_eq!(blocks[0].pool, Some(4));
        assert!(blocks[0].within_family_identical);
        assert_eq!(blocks[0].targets, ["spacetime-pgwire-rs"]);
        assert_eq!(blocks[0].exempt, ["spacetime-sdk-rs"]);
    }

    /// Consumers join `harness[].family` against each target's declared
    /// `fair.family`, and `harness[].targets` against the run's target list.
    /// Both keys are echoed, never re-derived, so the join cannot go stale.
    #[test]
    fn every_emitted_key_traces_back_to_a_declared_target() {
        let mut cache = target("spacetime-sdk-rs", "spacetimedb", 1, 1, "cache");
        cache.data_access = Some("in-process-cache".to_string());
        let targets = [
            target("drizzle-rs-sqlite", "sqlite", 1, 8, "WAL"),
            target("drizzle-rs-pg", "postgres", 1, 8, "stock"),
            target("spacetime-pgwire-rs", "spacetimedb", 1, 4, "stock"),
            cache,
        ];
        let blocks = harness(&targets).expect("harness");

        for block in &blocks {
            assert!(
                targets.iter().any(|t| t.fair.family == block.family),
                "family {} matches no target",
                block.family
            );
            for id in block.targets.iter().chain(&block.exempt) {
                assert!(
                    targets.iter().any(|t| &t.id == id),
                    "harness names {id}, which is not in the run"
                );
            }
        }
        // Every target is accounted for exactly once, compared or exempt.
        let named: Vec<&String> = blocks
            .iter()
            .flat_map(|b| b.targets.iter().chain(&b.exempt))
            .collect();
        assert_eq!(named.len(), targets.len());
    }

    /// Within-family identity is enforced per *run*, but a family can span runs:
    /// `targets.postgres.v1.json` and `targets.postgres-rust-orms.v1.json` both
    /// declare `family: postgres`, and outside the publish topology they execute
    /// as separate CI jobs producing separate artifacts. Each run would then
    /// check only its own shard and pass, and the drift would not surface until
    /// a consumer merged the two and marked the family unverified — after
    /// publish, in someone else's UI.
    ///
    /// So check the union of every checked-in spec here, where a mismatch fails
    /// CI at source instead.
    #[test]
    fn every_checked_in_spec_agrees_with_its_family_across_files() {
        let spec_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("bench")
            .join("spec");

        let mut all: Vec<Target> = Vec::new();
        let mut files = 0;
        for entry in std::fs::read_dir(&spec_dir).expect("read bench/spec") {
            let path = entry.expect("dir entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.starts_with("targets.") || !name.ends_with(".json") {
                continue;
            }
            let body = std::fs::read_to_string(&path).expect("read target spec");
            all.extend(
                serde_json::from_str::<Vec<Target>>(&body)
                    .unwrap_or_else(|err| panic!("{}: {err}", path.display())),
            );
            files += 1;
        }
        assert!(files >= 8, "expected every targets.*.json, found {files}");

        let blocks = harness(&all).unwrap_or_else(|err| {
            panic!(
                "checked-in specs declare an unfair comparison, which would only \
                 have surfaced after publish in the parallel CI topology: {}",
                err.msg
            )
        });

        // The case this exists for: one family, two spec files, one harness.
        let postgres = blocks
            .iter()
            .find(|block| block.family == "postgres")
            .expect("postgres family");
        assert!(
            postgres.targets.len() > 5,
            "postgres should span both Rust spec files, found {:?}",
            postgres.targets
        );
        assert!(postgres.within_family_identical);
    }

    #[test]
    fn a_family_of_only_exempt_targets_claims_no_harness() {
        let mut cache = target("spacetime-sdk-rs", "spacetimedb", 1, 1, "cache");
        cache.data_access = Some("in-process-cache".to_string());
        let blocks = harness(&[cache]).expect("vacuous check");

        assert!(!blocks[0].within_family_identical);
        assert!(blocks[0].workers.is_none());
        assert!(blocks[0].pool.is_none());
        assert!(blocks[0].tuning.is_none());
        assert_eq!(blocks[0].exempt, ["spacetime-sdk-rs"]);
    }
}
