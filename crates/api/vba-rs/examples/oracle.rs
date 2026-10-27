//! Command line of the differential-testing (oracle) tooling.
//!
//! ```text
//! cargo run -p vba-rs --example oracle -- stats
//! cargo run -p vba-rs --example oracle -- skiplist
//! cargo run -p vba-rs --example oracle -- prepare <bundle-dir>
//! cargo run -p vba-rs --example oracle -- report <dir | results.jsonl [<environment.json>]>
//! cargo run -p vba-rs --example oracle -- apply  <dir | results.jsonl <environment.json>> [--allow-locale]
//! cargo run -p vba-rs --example oracle -- fake-run <dir> [--perturb]
//! ```
//!
//! See `docs/tooling/differential-testing.md`. The Windows side (`tests/oracle/`)
//! is untested; everything here is tested by `tests/oracle_tooling.rs`.

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oracle_support as o;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn skip_list(oracle: &Path) -> BTreeMap<String, (String, String)> {
    std::fs::read_to_string(oracle.join("oracle-skip.txt"))
        .map(|t| o::parse_skip_list(&t))
        .unwrap_or_default()
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = root();
    let oracle = root.join("tests/oracle");
    let progs = o::load_corpus(&root.join("tests/vba"))?;
    let skip = skip_list(&oracle);
    match args.first().map(String::as_str) {
        Some("stats") => {
            let mut ok = 0;
            let mut by_cat = BTreeMap::<&str, usize>::new();
            for p in &progs {
                match o::classify(p) {
                    Ok(_) if !skip.contains_key(&p.id) => ok += 1,
                    Ok(_) => *by_cat.entry("manual").or_default() += 1,
                    Err(r) => *by_cat.entry(r[0].category).or_default() += 1,
                }
            }
            println!("programs: {}\ncapturable: {ok}", progs.len());
            for (c, n) in by_cat {
                println!("not capturable ({c}): {n}");
            }
        }
        Some("skiplist") => {
            let text = o::render_skip_list(&progs, &skip);
            std::fs::write(oracle.join("oracle-skip.txt"), text).map_err(|e| e.to_string())?;
        }
        Some("prepare") => {
            let out = PathBuf::from(args.get(1).ok_or("prepare needs an output directory")?);
            let m = o::write_bundle(&progs, &skip, &oracle.join("OracleHarness.bas"), &out)?;
            let skip_file = oracle.join("oracle-skip.txt");
            if skip_file.exists() {
                std::fs::copy(&skip_file, out.join("oracle-skip.txt"))
                    .map_err(|e| e.to_string())?;
            }
            o::write_lab_files(&oracle, &out)?;
            let all = m["programs"].as_array().map_or(0, Vec::len);
            let cap = m["programs"].as_array().map_or(0, |a| {
                a.iter().filter(|p| p["status"] == "capturable").count()
            });
            println!(
                "bundle written to {}: {cap} capturable of {all}",
                out.display()
            );
        }
        Some("fake-run") => {
            let out = PathBuf::from(args.get(1).ok_or("fake-run needs an output directory")?);
            let mode = if args.iter().any(|a| a == "--perturb") {
                o::FakeMode::Perturbed
            } else {
                o::FakeMode::Echo
            };
            let (results, env) = o::fake_oracle(&progs, &skip, mode)?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let id = o::FAKE_ENV_ID;
            std::fs::write(out.join(format!("results-{id}.jsonl")), results)
                .map_err(|e| e.to_string())?;
            std::fs::write(out.join(format!("environment-{id}.json")), env)
                .map_err(|e| e.to_string())?;
            println!(
                "fake results ({mode:?}) written to {}; they say nothing about real VBA and `apply` refuses them",
                out.display()
            );
        }
        Some(cmd @ ("report" | "apply")) => {
            let first = PathBuf::from(args.get(1).ok_or("needs a results file or directory")?);
            // A directory holds one or more results/environment pairs, as the
            // driver writes them; the pairs are processed in environment-id order.
            let pairs = if first.is_dir() {
                o::find_result_pairs(&first)?
            } else {
                let env = args
                    .get(2)
                    .filter(|a| !a.starts_with("--"))
                    .map(PathBuf::from);
                vec![(first, env.unwrap_or_default())]
            };
            let allow = args.iter().any(|a| a == "--allow-locale");
            // Each environment is applied to the corpus as the previous ones left
            // it, so the program text is reloaded between pairs.
            for (results_path, env_path) in pairs {
                let progs = o::load_corpus(&root.join("tests/vba"))?;
                let results = o::parse_results(
                    &std::fs::read_to_string(&results_path).map_err(|e| e.to_string())?,
                )?;
                let env = if env_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(o::parse_environment(
                        &std::fs::read_to_string(&env_path).map_err(|e| e.to_string())?,
                    )?)
                };
                let rows = o::compare(&progs, &skip, &results);
                if cmd == "report" {
                    print!("{}", o::render_report(&rows, env.as_ref()));
                } else {
                    let env = env.ok_or("apply needs an environment file")?;
                    let out = o::apply(&progs, &rows, &env, &oracle, allow)?;
                    println!(
                        "{}: upgraded {}, already oracle {}, environment row added: {}, mismatch report: {:?}",
                        env.env_id,
                        out.upgraded.len(),
                        out.already_oracle.len(),
                        out.environment_row_added,
                        out.mismatch_report
                    );
                }
            }
        }
        _ => {
            return Err(
                "usage: oracle stats|skiplist|prepare|report|apply|fake-run (see the file header)"
                    .into(),
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
