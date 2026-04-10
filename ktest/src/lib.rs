pub use insta::{self, assert_snapshot};
pub use similar_asserts;
pub mod util;

pub use util::JustId;

#[macro_export]
macro_rules! assert_dotgraph {
    ($cmd:expr) => {
        let kb = krates::Builder::new();
        $crate::assert_dotgraph!(kb, $cmd);
    };

    ($path:literal, $builder:expr) => {{
        let grafs = $crate::util::build($path, $builder).unwrap();
        $crate::do_assert!(&grafs.dotgraph());
    }};

    (default $path:literal) => {
        $crate::assert_dotgraph!($path, krates::Builder::new());
    };

    ($builder:expr, $cmd:expr) => {
        let md: krates::Krates<$crate::JustId> = $builder.build($cmd, krates::NoneFilter).unwrap();
        $crate::do_assert!(&krates::petgraph::dot::Dot::new(md.graph()).to_string());
    };
}

#[macro_export]
macro_rules! do_assert {
    ($val:expr) => {
        let res = std::panic::catch_unwind(|| $crate::assert_snapshot!($val));

        if let Err(err) = res {
            let fname = $crate::insta::_function_name!();
            let mname = $crate::insta::_macro_support::module_path!();

            let (_, fname) = fname.rsplit_once("::").unwrap();
            let mut name = String::with_capacity(64);
            for comp in mname.split("::") {
                if !name.is_empty() {
                    name.push_str("__");
                }

                name.push_str(comp);
            }

            name.push_str("__");
            name.push_str(fname);

            let mut root =
                std::fs::canonicalize(std::path::Path::new("./tests/snapshots")).unwrap();
            root.push(name);

            // AFAICT dot can't take stdin
            root.set_extension("dot");
            std::fs::write(&root, $val.as_bytes()).unwrap();

            let out = std::process::Command::new("dot")
                .arg("-Tsvg")
                .arg(&root)
                .output()
                .unwrap();

            root.set_extension("svg");
            std::fs::write(&root, out.stdout).unwrap();

            eprintln!("file://{}", root.to_str().unwrap());

            std::panic::resume_unwind(err);
        }
    };
}
