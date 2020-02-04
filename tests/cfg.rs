mod util;

use krates::cfg_expr::targets;
use util::{build, cmp};

#[test]
fn ignores_non_linux() {
    let mut kb = krates::Builder::new();
    kb.include_targets(targets::ALL.iter().filter_map(|ti| {
        if ti.os == Some(targets::Os::linux) {
            Some((ti.triple, Vec::new()))
        } else {
            None
        }
    }));

    let grafs = build("all-features.json", kb).unwrap();

    let targets: Vec<_> = targets::ALL
        .iter()
        .filter(|ti| ti.os == Some(targets::Os::linux))
        .collect();

    cmp(
        grafs,
        |_| false,
        |ef| {
            if let Some(cfg) = ef.cfg {
                if cfg.starts_with("cfg(") {
                    let expr = krates::cfg_expr::Expression::parse(cfg).unwrap();

                    for ti in &targets {
                        if expr.eval(|pred| match pred {
                            krates::cfg_expr::Predicate::Target(tp) => tp.matches(&ti),
                            _ => false,
                        }) {
                            println!("{} matched {}", cfg, ti.triple);
                            return false;
                        }
                    }

                    true
                } else {
                    !targets.iter().any(|ti| ti.triple == cfg)
                }
            } else {
                false
            }
        },
    );
}

#[test]
fn ignores_non_tier1() {
    let mut kb = krates::Builder::new();

    let targets = vec![
        targets::get_target_by_triple("i686-pc-windows-gnu").unwrap(),
        targets::get_target_by_triple("i686-pc-windows-msvc").unwrap(),
        targets::get_target_by_triple("i686-unknown-linux-gnu").unwrap(),
        targets::get_target_by_triple("x86_64-apple-darwin").unwrap(),
        targets::get_target_by_triple("x86_64-pc-windows-gnu").unwrap(),
        targets::get_target_by_triple("x86_64-pc-windows-msvc").unwrap(),
        targets::get_target_by_triple("x86_64-unknown-linux-gnu").unwrap(),
    ];

    kb.include_targets(targets.iter().map(|ti| (ti.triple, vec![])));

    let grafs = build("all-features.json", kb).unwrap();

    cmp(
        grafs,
        |_| false,
        |ef| {
            if let Some(cfg) = ef.cfg {
                if cfg.starts_with("cfg(") {
                    let expr = krates::cfg_expr::Expression::parse(cfg).unwrap();

                    for ti in &targets {
                        if expr.eval(|pred| match pred {
                            krates::cfg_expr::Predicate::Target(tp) => tp.matches(&ti),
                            _ => false,
                        }) {
                            println!("{} matched {}", cfg, ti.triple);
                            return false;
                        }
                    }

                    true
                } else {
                    !targets.iter().any(|ti| ti.triple == cfg)
                }
            } else {
                false
            }
        },
    );
}

#[test]
fn ignores_non_wasm() {
    let mut kb = krates::Builder::new();

    let targets = vec![targets::get_target_by_triple("wasm32-unknown-unknown").unwrap()];

    kb.include_targets(targets.iter().map(|ti| (ti.triple, vec![])));

    let grafs = build("all-features.json", kb).unwrap();

    cmp(
        grafs,
        |_| false,
        |ef| {
            if let Some(cfg) = ef.cfg {
                if cfg.starts_with("cfg(") {
                    let expr = krates::cfg_expr::Expression::parse(cfg).unwrap();

                    for ti in &targets {
                        if expr.eval(|pred| match pred {
                            krates::cfg_expr::Predicate::Target(tp) => tp.matches(&ti),
                            _ => false,
                        }) {
                            println!("{} matched {}", cfg, ti.triple);
                            return false;
                        }
                    }

                    true
                } else {
                    !targets.iter().any(|ti| ti.triple == cfg)
                }
            } else {
                false
            }
        },
    );
}
