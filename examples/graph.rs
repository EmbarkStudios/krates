use clap::Parser;
use std::fmt;

/// Simple program to greet a person
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    manifest_path: String,
    #[arg(long)]
    features: Vec<String>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
}

pub struct Simple {
    id: krates::Kid,
    //features: HashMap<String, Vec<String>>,
}

pub type Graph = krates::Krates<Simple>;

impl From<krates::cm::Package> for Simple {
    fn from(pkg: krates::cm::Package) -> Self {
        Self {
            id: pkg.id,
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
        cmd.manifest_path(args.manifest_path);
        cmd
    };

    let builder = krates::Builder::new();
    let graph: Graph = builder.build(cmd, krates::NoneFilter).unwrap();

    let dot = krates::petgraph::dot::Dot::new(graph.graph()).to_string();

    use std::io::Write;
    std::io::stdout().write_all(dot.as_bytes()).unwrap();
}
