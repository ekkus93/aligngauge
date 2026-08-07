//! Core contracts for `AlignGauge`.

pub mod atomic;
pub mod config;
pub mod error;
pub mod json;
pub mod model;
pub mod targeted;

pub use atomic::{AtomicPublisher, OutputBundle, PublicationHook, PublicationStep};
pub use config::{
    CONFIG_SCHEMA_VERSION, ConfigOverrides, Environment, LogFormat, MapEnvironment,
    ProcessEnvironment, ResolvedConfig, resolve_config,
};
pub use error::{AlignGaugeError, ErrorCategory};
pub use json::{JsonValue, ToJson};
pub use model::{
    AlignmentCounters, Availability, BuildInfo, CoveragePolicy, CoverageSummary, InputIdentity,
    MateOverlapPolicy, MetricDefinition, PerReferenceCounters, PerReferenceCoverageSummary,
    Provenance, RecordInclusion, Summary, SystemInfo, Warning,
};
pub use targeted::{PerTargetCoverageSummary, TargetedCoverageSummary, ZeroCoverageRunSummary};
