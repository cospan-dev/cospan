//! Measure the on-disk size of importing cospan's own git history into
//! a fresh `FsStore` using the per-file content-addressed schema tree
//! (panproto issue #49).
//!
//! Run from the cospan workspace root:
//!
//! ```text
//! cargo run --release --example measure_cospan_import -- /path/to/cospan/.git [revspec]
//! ```
//!
//! Default `revspec` is `HEAD`. The importer parses every blob along
//! the walked revisions, stores per-file `FileSchemaObject` leaves,
//! and assembles `SchemaTreeObject` nodes per commit. At the end the
//! example prints an object count and total byte size and tabulates a
//! small histogram by object type.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use panproto_core::vcs::{FsStore, Object};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let git_dir = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "/Users/awhite48/Projects/cospan/.git".into()),
    );
    let revspec = args.get(2).cloned().unwrap_or_else(|| "HEAD".into());
    // Optional `--max-commits=N` cap on how many commits to actually import,
    // counting back from `revspec`. The remaining ancestors are pre-populated
    // in the `known` map so the importer hides them from the walk.
    let max_commits: Option<usize> = args
        .iter()
        .find_map(|a| a.strip_prefix("--max-commits="))
        .and_then(|n| n.parse().ok());

    let git_repo = git2::Repository::open(&git_dir)?;
    let tmp = tempfile::tempdir()?;
    let mut store = FsStore::init(tmp.path())?;

    println!("git_dir     : {}", git_dir.display());
    println!("revspec     : {}", revspec);
    println!("max_commits : {:?}", max_commits);
    println!("store       : {}", tmp.path().display());

    // Build a `known` map that covers everything beyond `max_commits` so the
    // walk only imports the top N commits.
    let mut known: HashMap<git2::Oid, panproto_core::vcs::ObjectId> = HashMap::new();
    let mut commits_walked = 0usize;
    if let Some(cap) = max_commits {
        let head = git_repo.revparse_single(&revspec)?.peel_to_commit()?.id();
        let mut walk = git_repo.revwalk()?;
        walk.push(head)?;
        let dummy: panproto_core::vcs::ObjectId =
            panproto_core::vcs::ObjectId::from_bytes([0u8; 32]);
        for oid in walk {
            let oid = oid?;
            commits_walked += 1;
            if commits_walked > cap {
                known.insert(oid, dummy);
            }
        }
        println!(
            "walked      : {commits_walked} commits, hiding {} ancestors",
            known.len()
        );
    }

    let start = Instant::now();
    let result =
        panproto_git::import_git_repo_incremental(&git_repo, &mut store, &revspec, &known)?;
    let elapsed = start.elapsed();

    println!("import   : {elapsed:?}");
    println!("head_id  : {}", result.head_id);

    // Walk the on-disk store and tabulate.
    let mut total_bytes: u64 = 0;
    let mut total_count: u64 = 0;
    let mut by_type: HashMap<&'static str, (u64, u64)> = HashMap::new();

    let objects_root = tmp.path().join(".panproto").join("objects");
    walk(&objects_root, &mut |path, len| {
        total_bytes += len;
        total_count += 1;
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(obj) = rmp_serde::from_slice::<Object>(&bytes)
        {
            let e = by_type.entry(obj.type_name()).or_insert((0, 0));
            e.0 += 1;
            e.1 += len;
        }
    })?;

    println!();
    println!("== fresh FsStore (per-file content addressed) ==");
    println!("object count : {total_count}");
    println!(
        "total bytes  : {} ({:.2} MiB)",
        total_bytes,
        total_bytes as f64 / (1024.0 * 1024.0)
    );
    println!();
    println!(
        "{:<20} {:>10} {:>14} {:>10}",
        "type", "count", "bytes", "MiB"
    );
    let mut rows: Vec<(&'static str, u64, u64)> =
        by_type.into_iter().map(|(k, (c, b))| (k, c, b)).collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.2));
    for (ty, count, bytes) in rows {
        println!(
            "{:<20} {:>10} {:>14} {:>10.2}",
            ty,
            count,
            bytes,
            bytes as f64 / (1024.0 * 1024.0)
        );
    }

    Ok(())
}

fn walk(
    dir: &std::path::Path,
    visit: &mut dyn FnMut(&std::path::Path, u64),
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            for e in std::fs::read_dir(&p)? {
                stack.push(e?.path());
            }
        } else {
            let meta = std::fs::metadata(&p)?;
            visit(&p, meta.len());
        }
    }
    Ok(())
}
