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
    entries!(map, self, resolve, workspace_root, target_directory);
    entry!(map, self, workspace_metadata, "metadata");
    entry!(map, self, version);
});

map!(Package, map, self, {
    entries!(
        map,
        self,
        name,
        version,
        authors,
        id,
        source,
        description,
        dependencies,
        license,
        license_file,
        targets,
        features,
        manifest_path,
        categories,
        keywords,
        readme,
        repository,
        homepage,
        documentation,
        edition
    );
    if !self.metadata.is_null() {
        entry!(map, self, metadata);
    }
    entries!(map, self, links, publish, default_run, rust_version);
});

map!(Resolve, map, self, {
    entries!(map, self, nodes, root);
});

map!(Node, map, self, {
    entries!(map, self, id, deps, dependencies, features);
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
        let s = match self {
            Self::Normal => "normal",
            Self::Development => "dev",
            Self::Build => "build",
        };

        serializer.serialize_str(s)
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
        optional,
        uses_default_features,
        features,
        target,
        rename,
        registry,
        path
    );
});

map!(Target, map, self, {
    entries!(
        map,
        self,
        name,
        kind,
        crate_types,
        required_features,
        src_path,
        edition,
        doctest,
        doctest,
        test,
        doc
    );
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
