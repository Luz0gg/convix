use crate::{Converter, Error, Format, JobContext, OperationDescriptor, Result};
use std::{path::PathBuf, sync::Arc};

/// All converters known to the app. Built once at startup, then read-only.
#[derive(Default, Clone)]
pub struct Registry {
    converters: Vec<Arc<dyn Converter>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, converter: impl Converter + 'static) -> &mut Self {
        self.converters.push(Arc::new(converter));
        self
    }

    pub fn converters(&self) -> impl Iterator<Item = &Arc<dyn Converter>> {
        self.converters.iter()
    }

    /// Every operation offered for `input`, across all converters.
    /// E.g. a PNG gets image conversions *and* "images → PDF" from the PDF converter.
    pub fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        self.converters.iter().flat_map(|c| c.operations_for(input)).collect()
    }

    pub fn find(&self, converter_id: &str) -> Option<&Arc<dyn Converter>> {
        self.converters.iter().find(|c| c.id() == converter_id)
    }

    /// Dispatch to the converter named by the operation id prefix.
    pub fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>> {
        let converter_id = ctx.operation_id.split('.').next().unwrap_or_default();
        let converter = self
            .find(converter_id)
            .ok_or_else(|| Error::UnknownOperation(ctx.operation_id.to_owned()))?;
        ctx.cancel.check()?;
        converter.run(ctx)
    }
}
