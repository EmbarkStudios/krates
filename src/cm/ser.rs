//! Provides serialization of cm types

use super::*;
use serde::ser::{Serialize, SerializeMap, Serializer};

impl Serialize for PackageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.repr)
    }
}

impl Serialize for Source {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.repr)
    }
}

macro_rules! entry {
    ($map:expr, $self:ident, $field:ident, $key:expr) => {
        $map.serialize_entry($key, &$self.$field)?;
    };
    ($map:expr, $self:ident, $field:ident) => {
        entry!($map, $self, $field, stringify!($field))
    };
}

macro_rules! entries {
    ($map:expr, $self:ident, $($fields:ident),+) => {
        $(
            $map.serialize_entry(stringify!($fields), &$self.$fields)?;
        )+
    }
}

macro_rules! map {
    ($kind:ty, $map:ident, $self:ident, $func:block) => {
        impl Serialize for $kind {
            fn serialize<S>(&$self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let mut $map = serializer.serialize_map(None)?;

                $func

                $map.end()
            }
        }
    };
}

map!(Metadata, map, self, {
    entries!(map, self, packages, workspace_members);
    if let Some(wdm) = &self.workspace_default_members.0 {
        map.serialize_entry("workspace_default_members", wdm)?;
    }

    entries!(map, self, resolve, target_directory);

    if let Some(build_directory) = &self.build_directory {
        map.serialize_entry("build_directory", build_directory)?;
    }

    entries!(map, self, version, workspace_root);
    entry!(map, self, workspace_metadata, "metadata");
});

map!(Package, map, self, {
    entries!(
        map,
        self,
        name,
        version,
        id,
        license,
        license_file,
        description,
        source,
        dependencies,
        targets,
        features,
        manifest_path,
        metadata,
        publish,
        authors,
        categories,
        keywords,
        readme,
        repository,
        homepage,
        documentation,
        edition,
        links,
        default_run
    );

    if let Some(rv) = &self.rust_version {
        let mut s = format!("{}.{}", rv.major, rv.minor);
        if rv.patch > 0 {
            use std::fmt::Write;
            write!(&mut s, ".{}", rv.patch).unwrap();
        }
        map.serialize_entry("rust_version", &s)?;
    } else {
        map.serialize_entry("rust_version", &self.rust_version)?;
    }
});

map!(Resolve, map, self, {
    entries!(map, self, nodes, root);
});

map!(Node, map, self, {
    entries!(map, self, id, dependencies, deps, features);
});

map!(NodeDep, map, self, {
    entries!(map, self, name, pkg, dep_kinds);
});

map!(DepKindInfo, map, self, {
    entries!(map, self, kind, target);
});

impl Serialize for DependencyKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Normal => serializer.serialize_none(),
            Self::Development => serializer.serialize_str("dev"),
            Self::Build => serializer.serialize_str("build"),
        }
    }
}

map!(Dependency, map, self, {
    entries!(
        map,
        self,
        name,
        source,
        req,
        kind,
        rename,
        optional,
        uses_default_features,
        features,
        target,
        registry
    );

    if let Some(p) = &self.path {
        map.serialize_entry("path", p)?;
    }
});

map!(Target, map, self, {
    entries!(map, self, kind, crate_types, name, src_path, edition);

    if !self.required_features.is_empty() {
        entry!(map, self, required_features);
    }

    entries!(map, self, doc, doctest, test);
});

impl Serialize for Edition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Self::E2015 => "2015",
            Self::E2018 => "2018",
            Self::E2021 => "2021",
            Self::E2024 => "2024",
        };

        serializer.serialize_str(s)
    }
}

impl Serialize for TargetKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Self::Example => "example",
            Self::Test => "test",
            Self::Bench => "bench",
            Self::CustomBuild => "custom-build",
            Self::Bin => "bin",
            Self::Lib => "lib",
            Self::RLib => "rlib",
            Self::DyLib => "dylib",
            Self::CDyLib => "cdylib",
            Self::StaticLib => "staticlib",
            Self::ProcMacro => "proc-macro",
        };

        serializer.serialize_str(s)
    }
}

impl Serialize for CrateType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Self::Bin => "bin",
            Self::Lib => "lib",
            Self::RLib => "rlib",
            Self::DyLib => "dylib",
            Self::CDyLib => "cdylib",
            Self::StaticLib => "staticlib",
            Self::ProcMacro => "proc-macro",
        };

        serializer.serialize_str(s)
    }
}
