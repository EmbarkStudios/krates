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

pub(crate) struct ParsedFeature<'feat> {
    inner: &'feat str,
    kind: FeatureKind,
}

impl<'feat> ParsedFeature<'feat> {
    #[inline]
    pub(crate) fn feat(&self) -> Feature<'feat> {
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

impl<'feat> From<&'feat str> for ParsedFeature<'feat> {
    #[inline]
    fn from(f: &'feat str) -> Self {
        let kind = if f.starts_with("dep:") {
            FeatureKind::Krate
        } else if let Some(ind) = f.find("?/") {
            FeatureKind::Weak(ind)
        } else if let Some(ind) = f.find("/") {
            FeatureKind::Strong(ind)
        } else {
            FeatureKind::Simple
        };

        Self { inner: f, kind }
    }
}
