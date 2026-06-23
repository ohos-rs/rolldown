use arcstr::ArcStr;
use rolldown_common::{ExportsKind, ImportKind, ModuleType};
use rolldown_error::BuildDiagnostic;

#[derive(Debug)]
pub struct ModuleSourceScanOutput {
  pub modules: Vec<ModuleSourceScanModule>,
  pub warnings: Vec<BuildDiagnostic>,
}

impl ModuleSourceScanOutput {
  pub fn is_empty(&self) -> bool {
    self.modules.is_empty()
  }
}

#[derive(Debug)]
pub struct ModuleSourceScanModule {
  pub id: ArcStr,
  pub source: ArcStr,
  pub module_type: ModuleType,
  pub exports_kind: ExportsKind,
  pub imports: Vec<ModuleSourceScanImport>,
}

#[derive(Debug)]
pub struct ModuleSourceScanImport {
  pub module_request: String,
  pub resolved_id: Option<ArcStr>,
  pub kind: ImportKind,
  pub importer_span: ModuleSourceScanSpan,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ModuleSourceScanSpan {
  pub start: u32,
  pub end: u32,
}
