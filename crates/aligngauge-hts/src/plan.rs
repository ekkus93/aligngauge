//! Minimal required-field planning for the v0.1 BAM reader.

use std::collections::{BTreeMap, BTreeSet};

use aligngauge_core::JsonValue;

/// A field that may be exposed by the validated BAM record boundary.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum RequiredField {
    /// SAM flags.
    Flags,
    /// Reference identifier and position.
    Coordinates,
    /// Mate reference identifier and position.
    MateCoordinates,
    /// Mapping quality.
    MappingQuality,
    /// Validated CIGAR facts and raw operations.
    Cigar,
    /// Read-group tag and declaration state.
    ReadGroup,
    /// Optional `NM` edit-distance tag.
    EditDistance,
    /// Optional `MD` mismatch descriptor.
    MismatchDescriptor,
    /// Decoded sequence bases. Materialized only by an explicit plan.
    Sequence,
    /// Optional Picard `XN` noise tag interpreted as integer one.
    NoiseTag,
    /// Base qualities.
    Qualities,
    /// Signed template length / TLEN.
    TemplateLength,
}

impl RequiredField {
    /// All fields in stable provenance order.
    pub const ALL: [Self; 12] = [
        Self::Flags,
        Self::Coordinates,
        Self::MateCoordinates,
        Self::MappingQuality,
        Self::Cigar,
        Self::ReadGroup,
        Self::EditDistance,
        Self::MismatchDescriptor,
        Self::Sequence,
        Self::NoiseTag,
        Self::Qualities,
        Self::TemplateLength,
    ];

    /// Stable machine-readable field name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flags => "flags",
            Self::Coordinates => "coordinates",
            Self::MateCoordinates => "mate_coordinates",
            Self::MappingQuality => "mapping_quality",
            Self::Cigar => "cigar",
            Self::ReadGroup => "read_group",
            Self::EditDistance => "nm",
            Self::MismatchDescriptor => "md",
            Self::Sequence => "sequence",
            Self::NoiseTag => "xn_noise_tag",
            Self::Qualities => "qualities",
            Self::TemplateLength => "template_length",
        }
    }
}

/// Immutable set of fields required by the selected v0.1 collectors.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FieldPlan {
    fields: BTreeSet<RequiredField>,
}

impl FieldPlan {
    /// Fields required by Milestone 4 counters.
    #[must_use]
    pub fn counters() -> Self {
        Self::from_fields([
            RequiredField::Flags,
            RequiredField::Coordinates,
            RequiredField::MateCoordinates,
            RequiredField::MappingQuality,
        ])
    }

    /// Fields required by Milestone 5 exact coverage.
    #[must_use]
    pub fn coverage() -> Self {
        Self::from_fields([
            RequiredField::Flags,
            RequiredField::Coordinates,
            RequiredField::Cigar,
        ])
    }

    /// Build the exact Samtools 1.24 stats SN/IS compatibility plan.
    #[must_use]
    pub fn samtools_stats() -> Self {
        Self::from_fields([
            RequiredField::Flags,
            RequiredField::Coordinates,
            RequiredField::MateCoordinates,
            RequiredField::MappingQuality,
            RequiredField::Cigar,
            RequiredField::EditDistance,
            RequiredField::Qualities,
            RequiredField::TemplateLength,
        ])
    }

    /// Build the Picard 3.4.0 reference-independent alignment-summary plan.
    #[must_use]
    pub fn picard_alignment_summary() -> Self {
        Self::from_fields([
            RequiredField::Flags,
            RequiredField::Coordinates,
            RequiredField::MappingQuality,
            RequiredField::Sequence,
            RequiredField::NoiseTag,
        ])
    }

    /// Build the Picard 3.4.0 default `ALL_READS` insert-size plan.
    #[must_use]
    pub fn picard_insert_size() -> Self {
        Self::from_fields([
            RequiredField::Flags,
            RequiredField::Coordinates,
            RequiredField::MateCoordinates,
            RequiredField::Cigar,
            RequiredField::TemplateLength,
        ])
    }

    /// Add optional tags used by diagnostic and later metric collectors.
    #[must_use]
    pub fn with_optional_tags(mut self) -> Self {
        self.fields.extend([
            RequiredField::ReadGroup,
            RequiredField::EditDistance,
            RequiredField::MismatchDescriptor,
        ]);
        self
    }

    /// Union two plans without introducing execution-backend dimensions.
    #[must_use]
    pub fn union(mut self, other: &Self) -> Self {
        self.fields.extend(other.fields.iter().copied());
        self
    }

    /// Whether a field is part of the resolved plan.
    #[must_use]
    pub fn requires(&self, field: RequiredField) -> bool {
        self.fields.contains(&field)
    }

    /// Stable provenance representation of every supported field decision.
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let fields = RequiredField::ALL
            .into_iter()
            .map(|field| {
                (
                    field.as_str().to_owned(),
                    JsonValue::Bool(self.requires(field)),
                )
            })
            .collect::<BTreeMap<_, _>>();

        JsonValue::Object(BTreeMap::from([
            (
                String::from("schema"),
                JsonValue::String(String::from("aligngauge-field-plan-v1")),
            ),
            (String::from("fields"), JsonValue::Object(fields)),
        ]))
    }

    fn from_fields(fields: impl IntoIterator<Item = RequiredField>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldPlan, RequiredField};

    #[test]
    fn plan_union_is_stable_and_has_no_backend_surface() {
        let plan = FieldPlan::counters()
            .union(&FieldPlan::coverage())
            .with_optional_tags();
        let json = plan.to_json().to_compact_string();

        assert!(plan.requires(RequiredField::Cigar));
        assert!(plan.requires(RequiredField::ReadGroup));
        assert!(!plan.requires(RequiredField::Sequence));
        assert!(!plan.requires(RequiredField::Qualities));
        assert!(!json.contains("backend"));
        assert!(!json.contains("gpu"));
        assert!(!json.contains("cuda"));
    }
}
