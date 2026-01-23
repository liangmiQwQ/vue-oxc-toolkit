use oxc_syntax::module_record::ModuleRecord;

pub trait Merge: Sized {
  fn merge_imports(&mut self, instance: Self);
  fn merge(&mut self, instance: Self);
}

impl Merge for ModuleRecord<'_> {
  fn merge(&mut self, instance: Self) {
    self.has_module_syntax |= instance.has_module_syntax;
    self.requested_modules.extend(instance.requested_modules);
    self.import_entries.extend(instance.import_entries);
    self
      .local_export_entries
      .extend(instance.local_export_entries);
    self
      .indirect_export_entries
      .extend(instance.indirect_export_entries);
    self
      .star_export_entries
      .extend(instance.star_export_entries);
    self.exported_bindings.extend(instance.exported_bindings);
    self.dynamic_imports.extend(instance.dynamic_imports);
    self.import_metas.extend(instance.import_metas);
  }

  fn merge_imports(&mut self, instance: Self) {
    self.has_module_syntax |= instance.has_module_syntax;
    self.requested_modules.extend(instance.requested_modules);
    self.import_entries.extend(instance.import_entries);
    self.dynamic_imports.extend(instance.dynamic_imports);
    self.import_metas.extend(instance.import_metas);
  }
}
