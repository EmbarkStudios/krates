use super::Error;
use std::path::PathBuf;
use std::{ffi::OsString, process::Command};

/// Cargo features flags
#[derive(Debug, Clone)]
pub enum CargoOpt {
    /// Run cargo with `--features-all`
    AllFeatures,
    /// Run cargo with `--no-default-features`
    NoDefaultFeatures,
    /// Run cargo with `--features <FEATURES>`
    SomeFeatures(Vec<String>),
}

/// A builder for configurating `cargo metadata` invocation.
#[derive(Debug, Clone, Default)]
pub struct MetadataCommand {
    /// Path to `cargo` executable.  If not set, this will use the
    /// the `$CARGO` environment variable, and if that is not set, will
    /// simply be `cargo`.
    cargo_path: Option<PathBuf>,
    /// Path to `Cargo.toml`
    manifest_path: Option<PathBuf>,
    /// Current directory of the `cargo metadata` process.
    current_dir: Option<PathBuf>,
    /// Output information only about workspace members and don't fetch dependencies.
    no_deps: bool,
    /// Collections of `CargoOpt::SomeFeatures(..)`
    features: Vec<String>,
    /// Latched `CargoOpt::AllFeatures`
    all_features: bool,
    /// Latched `CargoOpt::NoDefaultFeatures`
    no_default_features: bool,
    /// Arbitrary command line flags to pass to `cargo`.  These will be added
    /// to the end of the command line invocation.
    other_options: Vec<String>,
    /// Arbitrary environment variables to set when running `cargo`.  These will be merged into
    /// the calling environment, overriding any which clash.
    env: std::collections::BTreeMap<OsString, OsString>,
    /// Show stderr
    verbose: bool,
}

impl MetadataCommand {
    /// Creates a default `cargo metadata` command, which will look for
    /// `Cargo.toml` in the ancestors of the current directory.
    pub fn new() -> Self {
        Self::default()
    }
    /// Path to `cargo` executable.  If not set, this will use the
    /// the `$CARGO` environment variable, and if that is not set, will
    /// simply be `cargo`.
    pub fn cargo_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.cargo_path = Some(path.into());
        self
    }
    /// Path to `Cargo.toml`
    pub fn manifest_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.manifest_path = Some(path.into());
        self
    }
    /// Current directory of the `cargo metadata` process.
    pub fn current_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.current_dir = Some(path.into());
        self
    }
    /// Output information only about workspace members and don't fetch dependencies.
    pub fn no_deps(&mut self) -> &mut Self {
        self.no_deps = true;
        self
    }
    /// Which features to include.
    ///
    /// Call this multiple times to specify advanced feature configurations:
    ///
    /// ```no_run
    /// # use cargo_metadata::{CargoOpt, MetadataCommand};
    /// MetadataCommand::new()
    ///     .features(CargoOpt::NoDefaultFeatures)
    ///     .features(CargoOpt::SomeFeatures(vec!["feat1".into(), "feat2".into()]))
    ///     .features(CargoOpt::SomeFeatures(vec!["feat3".into()]))
    ///     // ...
    ///     # ;
    /// ```
    ///
    /// # Panics
    ///
    /// `cargo metadata` rejects multiple `--no-default-features` flags. Similarly, the `features()`
    /// method panics when specifying multiple `CargoOpt::NoDefaultFeatures`:
    ///
    /// ```should_panic
    /// # use cargo_metadata::{CargoOpt, MetadataCommand};
    /// MetadataCommand::new()
    ///     .features(CargoOpt::NoDefaultFeatures)
    ///     .features(CargoOpt::NoDefaultFeatures) // <-- panic!
    ///     // ...
    ///     # ;
    /// ```
    ///
    /// The method also panics for multiple `CargoOpt::AllFeatures` arguments:
    ///
    /// ```should_panic
    /// # use cargo_metadata::{CargoOpt, MetadataCommand};
    /// MetadataCommand::new()
    ///     .features(CargoOpt::AllFeatures)
    ///     .features(CargoOpt::AllFeatures) // <-- panic!
    ///     // ...
    ///     # ;
    /// ```
    pub fn features(&mut self, features: CargoOpt) -> &mut Self {
        match features {
            CargoOpt::SomeFeatures(features) => self.features.extend(features),
            CargoOpt::NoDefaultFeatures => {
                assert!(
                    !self.no_default_features,
                    "Do not supply CargoOpt::NoDefaultFeatures more than once!"
                );
                self.no_default_features = true;
            }
            CargoOpt::AllFeatures => {
                assert!(
                    !self.all_features,
                    "Do not supply CargoOpt::AllFeatures more than once!"
                );
                self.all_features = true;
            }
        }
        self
    }
    /// Arbitrary command line flags to pass to `cargo`.  These will be added
    /// to the end of the command line invocation.
    pub fn other_options(&mut self, options: impl Into<Vec<String>>) -> &mut Self {
        self.other_options = options.into();
        self
    }

    /// Arbitrary environment variables to set when running `cargo`.  These will be merged into
    /// the calling environment, overriding any which clash.
    ///
    /// Some examples of when you may want to use this:
    /// 1. Setting cargo config values without needing a .cargo/config.toml file, e.g. to set
    ///    `CARGO_NET_GIT_FETCH_WITH_CLI=true`
    /// 2. To specify a custom path to RUSTC if your rust toolchain components aren't laid out in
    ///    the way cargo expects by default.
    ///
    /// ```no_run
    /// # use cargo_metadata::{CargoOpt, MetadataCommand};
    /// MetadataCommand::new()
    ///     .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
    ///     .env("RUSTC", "/path/to/rustc")
    ///     // ...
    ///     # ;
    /// ```
    pub fn env<K: Into<OsString>, V: Into<OsString>>(
        &mut self,
        key: K,
        val: V,
    ) -> &mut MetadataCommand {
        self.env.insert(key.into(), val.into());
        self
    }

    /// Set whether to show stderr
    pub fn verbose(&mut self, verbose: bool) -> &mut MetadataCommand {
        self.verbose = verbose;
        self
    }

    /// Builds a command for `cargo metadata`.  This is the first
    /// part of the work of `exec`.
    pub fn cargo_command(&self) -> Command {
        let cargo = self
            .cargo_path
            .clone()
            .or_else(|| std::env::var("CARGO").map(PathBuf::from).ok())
            .unwrap_or_else(|| PathBuf::from("cargo"));
        let mut cmd = Command::new(cargo);
        cmd.args(["metadata", "--format-version", "1"]);

        if self.no_deps {
            cmd.arg("--no-deps");
        }

        if let Some(path) = self.current_dir.as_ref() {
            cmd.current_dir(path);
        }

        if !self.features.is_empty() {
            cmd.arg("--features").arg(self.features.join(","));
        }
        if self.all_features {
            cmd.arg("--all-features");
        }
        if self.no_default_features {
            cmd.arg("--no-default-features");
        }

        if let Some(manifest_path) = &self.manifest_path {
            cmd.arg("--manifest-path").arg(manifest_path.as_os_str());
        }
        cmd.args(&self.other_options);

        cmd.envs(&self.env);

        cmd
    }

    /// Parses `cargo metadata` output.  `data` must have been
    /// produced by a command built with `cargo_command`.
    pub fn parse<T: AsRef<str>>(data: T) -> Result<super::Metadata, Error> {
        let meta = serde_json::from_str(data.as_ref())?;
        Ok(meta)
    }

    /// Runs configured `cargo metadata` and returns parsed `Metadata`.
    pub fn exec(&self) -> Result<super::Metadata, Error> {
        let mut command = self.cargo_command();
        if self.verbose {
            command.stderr(std::process::Stdio::inherit());
        }
        let output = command.output()?;
        if !output.status.success() {
            return Err(Error::CargoMetadata {
                stderr: String::from_utf8(output.stderr)?,
            });
        }

        let stdout = std::str::from_utf8(&output.stdout)?
            .lines()
            .find(|line| line.starts_with('{'))
            .ok_or(Error::NoJson)?;
        Self::parse(stdout)
    }
}
