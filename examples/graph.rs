use clap::Parser;
use std::fmt;

/// Simple program to greet a person
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    manifest_path: Option<String>,
    #[arg(long)]
    features: Vec<String>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
    #[arg(long)]
    no_dev: bool,
    #[arg(long, conflicts_with = "manifest_path")]
    json: Option<String>,
}

pub struct Simple {
    id: krates::Kid,
    //features: HashMap<String, Vec<String>>,
}

pub type Graph = krates::Krates<Simple>;

impl From<krates::cm::Package> for Simple {
    fn from(pkg: krates::cm::Package) -> Self {
        Self {
            id: pkg.id.into(),
            //features: pkg.fee
        }
    }
}

impl fmt::Display for Simple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id.repr)
    }
}

fn main() {
    let args = Args::parse();

    let graph: Graph = if let Some(manifest_path) = args.manifest_path {
        let cmd = {
            let mut cmd = krates::Cmd::new();
            if args.all_features {
                cmd.all_features();
            }
            if args.no_default_features {
                cmd.no_default_features();
            }
            if !args.features.is_empty() {
                cmd.features(args.features);
            }
            cmd.manifest_path(manifest_path);
            cmd
        };

        let mut builder = krates::Builder::new();
        if args.no_dev {
            builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        }

        builder.build(cmd, krates::NoneFilter).unwrap()
    } else if let Some(json) = args.json {
        let mut builder = krates::Builder::new();
        if args.no_dev {
            builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
        }

        let json = std::fs::read(json).expect("failed to read json");
        let md: krates::cm::Metadata =
            serde_json::from_slice(&json).expect("failed to deserialize metadata from json");

        builder.build_with_metadata(md, krates::NoneFilter).unwrap()
    } else {
        panic!("must specify either --manifest-path or --json");
    };

    let dot = krates::petgraph::dot::Dot::new(graph.graph()).to_string();

    use std::io::Write;
    std::io::stdout().write_all(dot.as_bytes()).unwrap();
}
