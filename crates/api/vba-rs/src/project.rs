//! [`Project`]: loading, analysing, editing and writing a VBA project.

use std::path::Path;

use vba_diagnostics::{Diagnostic, Severity, render_plain};
use vba_format::{Limits, ProjectStreams, ReadMode, WriteOptions, WriteReport};
use vba_profile::Profile;
use vba_project::{Module, ModuleKind, ProjectVersion, VbaProject};

use crate::Error;

/// Name given to a project built from loose sources.
pub const DEFAULT_PROJECT_NAME: &str = "VBAProject";

/// A VBA project: modules plus the metadata and preserved bytes of the file
/// it came from.
///
/// Reading a `vbaProject.bin` keeps everything the model does not interpret,
/// so [`Project::to_bytes`] after [`Project::set_module_source`] changes the
/// edited module and nothing else (see `vba-format` for the supported edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    model: VbaProject,
}

impl Project {
    /// An empty project named [`DEFAULT_PROJECT_NAME`].
    #[must_use]
    pub fn new() -> Project {
        Project {
            model: VbaProject::new(DEFAULT_PROJECT_NAME),
        }
    }

    /// A project with one standard module.
    #[must_use]
    pub fn from_source(module_name: &str, source: &str) -> Project {
        let mut p = Project::new();
        p.model
            .modules
            .push(Module::new(module_name, ModuleKind::Standard, source));
        p
    }

    /// A project named `name` (the default `Err.Source` of its errors) made of
    /// module texts the embedding application holds itself: `(module name,
    /// kind, source)`. No file is involved. A duplicate module name is
    /// [`Error::Model`].
    ///
    /// ```
    /// use vba::{Engine, ModuleKind, Project, Variant};
    ///
    /// let project = Project::from_modules("App", [
    ///     ("Utils", ModuleKind::Standard, "Public Function One() As Long\n  One = 1\nEnd Function\n"),
    /// ]).unwrap();
    /// let out = Engine::new().run(&project, "One", vec![]).unwrap();
    /// assert_eq!(out.value, Variant::Long(1));
    /// ```
    pub fn from_modules<'a>(
        name: &str,
        modules: impl IntoIterator<Item = (&'a str, ModuleKind, &'a str)>,
    ) -> Result<Project, Error> {
        let mut p = Project {
            model: VbaProject::new(name),
        };
        for (module, kind, source) in modules {
            p.add_module(module, kind, source)?;
        }
        Ok(p)
    }

    /// Wraps an existing model.
    #[must_use]
    pub fn from_model(model: VbaProject) -> Project {
        Project { model }
    }

    /// Reads a `vbaProject.bin` compound file with the default
    /// [`Limits`] against hostile input.
    pub fn from_bytes(bytes: &[u8]) -> Result<Project, Error> {
        Project::from_bytes_with(bytes, &Limits::default())
    }

    /// Reads a `vbaProject.bin` with explicit limits.
    pub fn from_bytes_with(bytes: &[u8], limits: &Limits) -> Result<Project, Error> {
        Ok(Project {
            model: vba_format::read_project(bytes, limits).map_err(Error::Format)?,
        })
    }

    /// Reads a project from named streams (`VBA/dir`, `VBA/_VBA_PROJECT`,
    /// `VBA/<module>`, `PROJECT`, ...) that the embedding application
    /// extracted itself, for example from a database table. No container is
    /// involved; see `docs/formats/vba-format.md`.
    pub fn from_streams(streams: &ProjectStreams, limits: &Limits) -> Result<Project, Error> {
        Ok(Project {
            model: vba_format::read_project_from_streams(streams, ReadMode::Strict, limits)
                .map_err(Error::Format)?,
        })
    }

    /// Reads a `vbaProject.bin` file. This is the embedder reading a file; a
    /// VBA program gets no such access. A file larger than
    /// [`ReadLimits::max_container_bytes`](crate::ReadLimits) (256 MiB by
    /// default) is refused without reading it all.
    pub fn open(path: impl AsRef<Path>) -> Result<Project, Error> {
        Project::open_with(path, &Limits::default())
    }

    /// [`open`](Project::open) with explicit limits.
    pub fn open_with(path: impl AsRef<Path>, limits: &Limits) -> Result<Project, Error> {
        use std::io::Read;
        let file = std::fs::File::open(path.as_ref()).map_err(Error::Io)?;
        let mut bytes = Vec::new();
        // One byte more than the limit, so that an oversized file is seen
        // as such without being read to the end.
        file.take(limits.max_container_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(Error::Io)?;
        Project::from_bytes_with(&bytes, limits)
    }

    /// Adds the text of a standalone `.bas`, `.cls` or `.frm` file as a module
    /// (kind and name derived from the file name and its `VB_Name` attribute).
    pub fn add_source_file(&mut self, file_name: &str, text: &str) -> Result<(), Error> {
        let m =
            Module::from_source_file(file_name, text).map_err(|e| Error::Source(e.to_string()))?;
        self.model.add_module(m).map_err(Error::Model)
    }

    /// Adds a module built from parts.
    pub fn add_module(&mut self, name: &str, kind: ModuleKind, source: &str) -> Result<(), Error> {
        self.model
            .add_module(Module::new(name, kind, source))
            .map_err(Error::Model)
    }

    /// The project name (`Err.Source` default).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.model.name
    }

    /// The modules, in project order.
    #[must_use]
    pub fn modules(&self) -> &[Module] {
        &self.model.modules
    }

    /// The module with this name (case-insensitive).
    #[must_use]
    pub fn module(&self, name: &str) -> Option<&Module> {
        self.model.module(name)
    }

    /// The source text of a module.
    #[must_use]
    pub fn module_source(&self, name: &str) -> Option<&str> {
        self.model.module(name).map(|m| m.source.as_str())
    }

    /// Replaces the source of an existing module.
    pub fn set_module_source(&mut self, name: &str, source: &str) -> Result<(), Error> {
        self.model
            .set_module_source(name, source)
            .map_err(Error::Model)
    }

    /// The underlying model (`vba-project`), for everything not covered here.
    #[must_use]
    pub fn model(&self) -> &VbaProject {
        &self.model
    }

    /// Mutable access to the underlying model.
    pub fn model_mut(&mut self) -> &mut VbaProject {
        &mut self.model
    }

    /// Consumes the project, returning the model.
    #[must_use]
    pub fn into_model(self) -> VbaProject {
        self.model
    }

    /// Writes the project as a `vbaProject.bin`, reproducing every stream the
    /// model preserved.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.to_bytes_with(&WriteOptions::default())
    }

    /// Writes the project with explicit [`WriteOptions`].
    pub fn to_bytes_with(&self, options: &WriteOptions) -> Result<Vec<u8>, Error> {
        vba_format::write_project(&self.model, options).map_err(Error::Format)
    }

    /// Writes the project as a complete set of streams, with a report of
    /// which of them changed since the project was read, so an embedder can
    /// write back only those (and delete the ones the report lists but the
    /// set no longer contains).
    pub fn to_streams(
        &self,
        options: &WriteOptions,
    ) -> Result<(ProjectStreams, WriteReport), Error> {
        vba_format::write_project_streams(&self.model, options).map_err(Error::Format)
    }

    /// Version-related facts about the project: the `_VBA_PROJECT` version
    /// word, platform, code page, locale, reference hints and whether it is
    /// compiled-only. These are facts, not a detection of the producing
    /// Office build; see [`ProjectVersion::profile_hint`].
    #[must_use]
    pub fn version(&self) -> ProjectVersion {
        self.model.project_version()
    }

    /// Parses and analyses the project without running anything.
    #[must_use]
    pub fn analyze(&self, profile: &Profile) -> Report {
        self.analyze_with(profile, &vba_sema::NoLibraries, &[], false)
    }

    /// [`analyze`](Project::analyze) with a type provider and the host
    /// member names an engine was configured with.
    pub(crate) fn analyze_with(
        &self,
        profile: &Profile,
        provider: &dyn vba_sema::TypeProvider,
        host_members: &[String],
        strict_spec: bool,
    ) -> Report {
        let analysis = vba_sema::analyze_project_with_options(
            &self.model,
            profile,
            provider,
            host_members,
            strict_spec,
        );
        let rendered = analysis
            .diagnostics
            .iter()
            .map(|d| render_plain(d, &analysis.sources))
            .collect();
        Report {
            diagnostics: analysis.diagnostics,
            rendered,
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Project::new()
    }
}

/// The diagnostics of analysing a project, in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    diagnostics: Vec<Diagnostic>,
    rendered: Vec<String>,
}

impl Report {
    /// Every diagnostic.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether any diagnostic is an error (the project would not load).
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// The error diagnostics.
    #[must_use]
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    /// The warning diagnostics.
    #[must_use]
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    /// The diagnostics as plain text, one entry per diagnostic, in the same
    /// order as [`Report::diagnostics`].
    #[must_use]
    pub fn rendered(&self) -> &[String] {
        &self.rendered
    }

    /// The codes (`SEM###`, `PAR###`...) of the diagnostics, in order.
    #[must_use]
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code.0).collect()
    }
}
