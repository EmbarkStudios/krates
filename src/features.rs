pub(crate) enum Feature<'feat> {
    Krate(&'feat str),
    Weak {
        krate: &'feat str,
        feature: &'feat str,
    },
    Strong {
        krate: &'feat str,
        feature: &'feat str,
    },
    Simple(&'feat str),
}

enum FeatureKind {
    Krate,
    Weak(usize),
    Strong(usize),
    Simple,
}

pub(crate) struct ParsedFeature {
    inner: String,
    kind: FeatureKind,
}

impl ParsedFeature {
    fn feat(&self) -> Feature<'_> {
        match self.kind {
            FeatureKind::Krate => Feature::Krate(&self.inner[..4]),
            FeatureKind::Weak(ind) => Feature::Weak {
                krate: &self.inner[..ind],
                feature: &self.inner[ind + 2..],
            },
            FeatureKind::Strong(ind) => Feature::Strong {
                krate: &self.inner[..ind],
                feature: &self.inner[ind + 1..],
            },
            FeatureKind::Simple => Feature::Simple(&self.inner),
        }
    }
}

impl From<String> for ParsedFeature {
    fn from(f: String) -> Self {
        let kind = if f.starts_with("dep:") {
            FeatureKind::Krate
        } else if let Some(ind) = f.index_of("?/") {
            FeatureKind::Weak(ind)
        } else if let Some(ind) = f.index_of("/") {
            FeatureKind::Strong(ind)
        } else {
            FeatureKind::Simple
        };

        Self { inner: f, kind }
    }
}
