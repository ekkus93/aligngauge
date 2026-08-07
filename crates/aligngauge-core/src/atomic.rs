//! Fail-closed atomic output publication.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};

use crate::error::{AlignGaugeError, ErrorCategory};

static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);

/// Checkpoints exposed to deterministic fault-injection tests.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PublicationStep {
    /// Same-filesystem staging directory exists.
    StagingCreated,
    /// Required files have been completely written but not synchronized.
    RequiredFilesWritten,
    /// Required file contents have been synchronized.
    RequiredFilesSynced,
    /// `_SUCCESS` has been written and synchronized as the final staging file.
    SuccessMarkerWritten,
    /// Staging-directory metadata has been synchronized where supported.
    StagingMetadataSynced,
    /// All preconditions hold immediately before atomic rename.
    BeforeRename,
}

/// Hook invoked at each publication checkpoint.
pub trait PublicationHook {
    /// Observe a checkpoint or inject a typed failure.
    ///
    /// # Errors
    ///
    /// Returning an error aborts publication and triggers fail-closed cleanup.
    fn checkpoint(
        &mut self,
        step: PublicationStep,
        staging: &Path,
        destination: &Path,
    ) -> Result<(), AlignGaugeError>;
}

#[derive(Debug, Default)]
struct NoopHook;

impl PublicationHook for NoopHook {
    fn checkpoint(
        &mut self,
        _step: PublicationStep,
        _staging: &Path,
        _destination: &Path,
    ) -> Result<(), AlignGaugeError> {
        Ok(())
    }
}

/// Files to publish as one completed output directory.
#[derive(Debug, Clone)]
pub struct OutputBundle {
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl OutputBundle {
    /// Construct a bundle containing the two required canonical JSON files.
    #[must_use]
    pub fn new(summary_json: impl Into<Vec<u8>>, provenance_json: impl Into<Vec<u8>>) -> Self {
        Self {
            files: BTreeMap::from([
                (PathBuf::from("provenance.json"), provenance_json.into()),
                (PathBuf::from("summary.json"), summary_json.into()),
            ]),
        }
    }

    /// Insert an additional root-level file.
    ///
    /// # Errors
    ///
    /// Returns `output_io` when the name is unsafe, reserved, or duplicated.
    pub fn insert(
        &mut self,
        name: impl Into<PathBuf>,
        content: impl Into<Vec<u8>>,
    ) -> Result<(), AlignGaugeError> {
        let name = name.into();
        validate_file_name(&name)?;
        if self.files.contains_key(&name) {
            return Err(output_error(format!(
                "output bundle contains duplicate file '{}'",
                name.display()
            )));
        }
        self.files.insert(name, content.into());
        Ok(())
    }

    fn validate(&self) -> Result<(), AlignGaugeError> {
        for name in self.files.keys() {
            validate_file_name(name)?;
        }
        for required in ["summary.json", "provenance.json"] {
            if !self.files.contains_key(Path::new(required)) {
                return Err(output_error(format!(
                    "output bundle is missing required file '{required}'"
                )));
            }
        }
        Ok(())
    }
}

/// Publishes one output bundle with same-filesystem staging and atomic rename.
#[derive(Debug, Clone)]
pub struct AtomicPublisher {
    destination: PathBuf,
    preserve_failed_staging: bool,
}

impl AtomicPublisher {
    /// Construct an atomic publisher.
    #[must_use]
    pub fn new(destination: impl Into<PathBuf>, preserve_failed_staging: bool) -> Self {
        Self {
            destination: destination.into(),
            preserve_failed_staging,
        }
    }

    /// Final destination path.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Publish with the production no-op hook.
    ///
    /// # Errors
    ///
    /// Returns a typed error without exposing a partial destination.
    pub fn publish(&self, bundle: &OutputBundle) -> Result<(), AlignGaugeError> {
        self.publish_with_hook(bundle, &mut NoopHook)
    }

    /// Publish while invoking a deterministic observer/fault-injection hook.
    ///
    /// # Errors
    ///
    /// Returns a typed error without exposing a partial destination.
    pub fn publish_with_hook(
        &self,
        bundle: &OutputBundle,
        hook: &mut impl PublicationHook,
    ) -> Result<(), AlignGaugeError> {
        bundle.validate()?;
        if self.destination.exists() {
            return Err(AlignGaugeError::new(
                ErrorCategory::OutputExists,
                format!(
                    "output destination '{}' already exists",
                    self.destination.display()
                ),
            )
            .with_detail(
                "destination",
                self.destination.to_string_lossy().into_owned(),
            ));
        }

        let parent = parent_or_current(&self.destination);
        fs::create_dir_all(parent).map_err(|source| {
            output_error(format!(
                "failed to create output parent directory '{}'",
                parent.display()
            ))
            .with_source(source)
        })?;

        let staging = create_staging_directory(parent, &self.destination)?;
        let result = self.publish_in_staging(bundle, hook, &staging);
        if let Err(mut error) = result {
            if let Err(cleanup_error) = self.handle_failed_staging(&staging) {
                error = error.with_detail("cleanup_error", cleanup_error.render_human(false));
            }
            return Err(error);
        }
        Ok(())
    }

    fn publish_in_staging(
        &self,
        bundle: &OutputBundle,
        hook: &mut impl PublicationHook,
        staging: &Path,
    ) -> Result<(), AlignGaugeError> {
        hook.checkpoint(PublicationStep::StagingCreated, staging, &self.destination)?;

        for (name, content) in &bundle.files {
            write_new_file(&staging.join(name), content, false)?;
        }
        hook.checkpoint(
            PublicationStep::RequiredFilesWritten,
            staging,
            &self.destination,
        )?;

        for name in bundle.files.keys() {
            sync_file(&staging.join(name))?;
        }
        hook.checkpoint(
            PublicationStep::RequiredFilesSynced,
            staging,
            &self.destination,
        )?;

        write_new_file(&staging.join("_SUCCESS"), &[], true)?;
        hook.checkpoint(
            PublicationStep::SuccessMarkerWritten,
            staging,
            &self.destination,
        )?;

        sync_directory(staging)?;
        hook.checkpoint(
            PublicationStep::StagingMetadataSynced,
            staging,
            &self.destination,
        )?;
        hook.checkpoint(PublicationStep::BeforeRename, staging, &self.destination)?;

        fs::rename(staging, &self.destination).map_err(|source| {
            output_error(format!(
                "failed to atomically publish '{}' as '{}'",
                staging.display(),
                self.destination.display()
            ))
            .with_source(source)
        })?;
        Ok(())
    }

    fn handle_failed_staging(&self, staging: &Path) -> Result<(), AlignGaugeError> {
        if !staging.exists() {
            return Ok(());
        }
        let success = staging.join("_SUCCESS");
        if success.exists() {
            fs::remove_file(&success).map_err(|source| {
                output_error(format!(
                    "failed to remove incomplete success marker '{}'",
                    success.display()
                ))
                .with_source(source)
            })?;
        }

        if self.preserve_failed_staging {
            let failed = failed_staging_path(staging)?;
            write_new_file(&staging.join("_FAILED"), &[], true)?;
            sync_directory(staging)?;
            fs::rename(staging, &failed).map_err(|source| {
                output_error(format!(
                    "failed to preserve incomplete staging directory as '{}'",
                    failed.display()
                ))
                .with_source(source)
            })?;
            sync_directory(parent_of(&failed)?)?;
        } else {
            fs::remove_dir_all(staging).map_err(|source| {
                output_error(format!(
                    "failed to remove incomplete staging directory '{}'",
                    staging.display()
                ))
                .with_source(source)
            })?;
            sync_directory(parent_of(staging)?)?;
        }
        Ok(())
    }
}

fn create_staging_directory(parent: &Path, destination: &Path) -> Result<PathBuf, AlignGaugeError> {
    let destination_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("results");
    for _ in 0..100 {
        let id = unique_id()?;
        let staging = parent.join(format!(".{destination_name}.aligngauge-{id}.staging"));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        match builder.create(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(output_error(format!(
                    "failed to create staging directory '{}'",
                    staging.display()
                ))
                .with_source(source));
            }
        }
    }
    Err(output_error(
        "failed to allocate a unique staging directory after 100 attempts",
    ))
}

fn write_new_file(path: &Path, content: &[u8], synchronize: bool) -> Result<(), AlignGaugeError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).map_err(|source| {
        output_error(format!("failed to create output file '{}'", path.display()))
            .with_source(source)
    })?;
    file.write_all(content).map_err(|source| {
        output_error(format!("failed to write output file '{}'", path.display()))
            .with_source(source)
    })?;
    file.flush().map_err(|source| {
        output_error(format!("failed to flush output file '{}'", path.display()))
            .with_source(source)
    })?;
    if synchronize {
        file.sync_all().map_err(|source| {
            output_error(format!(
                "failed to synchronize output file '{}'",
                path.display()
            ))
            .with_source(source)
        })?;
    }
    Ok(())
}

fn sync_file(path: &Path) -> Result<(), AlignGaugeError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| {
            output_error(format!(
                "failed to synchronize output file '{}'",
                path.display()
            ))
            .with_source(source)
        })
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), AlignGaugeError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| {
            output_error(format!(
                "failed to synchronize directory metadata for '{}'",
                path.display()
            ))
            .with_source(source)
        })
}

#[cfg(not(unix))]
fn sync_directory(path: &Path) -> Result<(), AlignGaugeError> {
    Err(output_error(format!(
        "atomic directory publication is not implemented for this platform: '{}'",
        path.display()
    )))
}

fn validate_file_name(name: &Path) -> Result<(), AlignGaugeError> {
    let mut components = name.components();
    let valid = matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && name != Path::new("_SUCCESS")
        && name != Path::new("_FAILED");
    if !valid {
        return Err(output_error(format!(
            "output file name '{}' must be a safe, non-reserved root-level name",
            name.display()
        )));
    }
    Ok(())
}

fn parent_or_current(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn failed_staging_path(staging: &Path) -> Result<PathBuf, AlignGaugeError> {
    let parent = parent_of(staging)?;
    let name = staging
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("aligngauge-staging");
    Ok(parent.join(format!("{name}.failed")))
}

fn parent_of(path: &Path) -> Result<&Path, AlignGaugeError> {
    path.parent()
        .ok_or_else(|| output_error(format!("path '{}' has no parent directory", path.display())))
}

fn unique_id() -> Result<String, AlignGaugeError> {
    let sequence = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| output_error("system clock predates the Unix epoch").with_source(source))?
        .as_nanos();
    Ok(format!("{}-{nanos}-{sequence}", std::process::id()))
}

fn output_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::OutputIo, message)
}

#[cfg(test)]
mod tests {
    use super::{AtomicPublisher, OutputBundle, PublicationHook, PublicationStep, output_error};
    use crate::error::ErrorCategory;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn successful_publication_is_complete_and_atomic_to_observers() {
        let root = test_root("success");
        let destination = root.join("result");
        let publisher = AtomicPublisher::new(&destination, false);
        let bundle = OutputBundle::new(b"{\"summary\":true}\n", b"{\"provenance\":true}\n");
        let mut observer = DestinationMustNotExist;

        publisher
            .publish_with_hook(&bundle, &mut observer)
            .expect("publish bundle");

        assert!(destination.join("summary.json").is_file());
        assert!(destination.join("provenance.json").is_file());
        assert!(destination.join("_SUCCESS").is_file());
        assert!(!destination.join("_FAILED").exists());
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn every_pre_rename_checkpoint_fails_closed() {
        for step in [
            PublicationStep::StagingCreated,
            PublicationStep::RequiredFilesWritten,
            PublicationStep::RequiredFilesSynced,
            PublicationStep::SuccessMarkerWritten,
            PublicationStep::StagingMetadataSynced,
            PublicationStep::BeforeRename,
        ] {
            let root = test_root(&format!("fault-{step:?}"));
            let destination = root.join("result");
            let publisher = AtomicPublisher::new(&destination, false);
            let bundle = OutputBundle::new(b"{}\n", b"{}\n");
            let mut hook = FailAt(step);

            let error = publisher
                .publish_with_hook(&bundle, &mut hook)
                .expect_err("fault must abort publication");
            assert_eq!(error.category(), ErrorCategory::InternalInvariant);
            assert!(!destination.exists());
            assert_eq!(directory_entries(&root), Vec::<PathBuf>::new());
            fs::remove_dir_all(root).expect("remove test root");
        }
    }

    #[test]
    fn preserved_failure_is_clearly_incomplete() {
        let root = test_root("preserved");
        let destination = root.join("result");
        let publisher = AtomicPublisher::new(&destination, true);
        let bundle = OutputBundle::new(b"{}\n", b"{}\n");
        let mut hook = FailAt(PublicationStep::BeforeRename);

        publisher
            .publish_with_hook(&bundle, &mut hook)
            .expect_err("fault must abort publication");
        assert!(!destination.exists());
        let entries = directory_entries(&root);
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0]
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".staging.failed"))
        );
        assert!(entries[0].join("_FAILED").is_file());
        assert!(!entries[0].join("_SUCCESS").exists());
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn existing_destination_is_never_overwritten() {
        let root = test_root("exists");
        let destination = root.join("result");
        fs::create_dir(&destination).expect("create destination");
        fs::write(destination.join("keep"), b"unchanged").expect("write sentinel");
        let publisher = AtomicPublisher::new(&destination, false);
        let bundle = OutputBundle::new(b"{}\n", b"{}\n");

        let error = publisher
            .publish(&bundle)
            .expect_err("existing output fails");
        assert_eq!(error.category(), ErrorCategory::OutputExists);
        assert_eq!(
            fs::read(destination.join("keep")).expect("read sentinel"),
            b"unchanged"
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    struct DestinationMustNotExist;

    impl PublicationHook for DestinationMustNotExist {
        fn checkpoint(
            &mut self,
            _step: PublicationStep,
            _staging: &Path,
            destination: &Path,
        ) -> Result<(), crate::AlignGaugeError> {
            assert!(!destination.exists());
            Ok(())
        }
    }

    struct FailAt(PublicationStep);

    impl PublicationHook for FailAt {
        fn checkpoint(
            &mut self,
            step: PublicationStep,
            _staging: &Path,
            _destination: &Path,
        ) -> Result<(), crate::AlignGaugeError> {
            if step == self.0 {
                return Err(crate::AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    format!("injected failure at {step:?}"),
                ));
            }
            Ok(())
        }
    }

    fn test_root(label: &str) -> PathBuf {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "aligngauge-atomic-{label}-{}-{id}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test root");
        }
        fs::create_dir(&root).expect("create test root");
        root
    }

    fn directory_entries(path: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<_> = fs::read_dir(path)
            .expect("read directory")
            .map(|entry| entry.expect("read entry").path())
            .collect();
        entries.sort();
        entries
    }

    #[test]
    fn bundle_rejects_paths_and_reserved_names() {
        let mut bundle = OutputBundle::new(b"{}", b"{}");
        assert!(bundle.insert("../escape", b"").is_err());
        assert!(bundle.insert("nested/file", b"").is_err());
        assert!(bundle.insert("_SUCCESS", b"").is_err());
        let error = output_error("test");
        assert_eq!(error.category(), ErrorCategory::OutputIo);
    }
}
