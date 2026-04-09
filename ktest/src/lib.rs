pub use insta::assert_snapshot;
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
        $crate::assert_snapshot!(grafs.dotgraph());
    }};

    (default $path:literal) => {
        $crate::assert_dotgraph!($path, krates::Builder::new());
    };

    ($builder:expr, $cmd:expr) => {
        let md: krates::Krates<$crate::JustId> = $builder.build($cmd, krates::NoneFilter).unwrap();
        $crate::assert_snapshot!(krates::petgraph::dot::Dot::new(md.graph()).to_string());
    };
}
