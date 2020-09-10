// This file is automatically @generated by fathom 0.1.0
// It is not intended for manual editing.

//! Test a singleton struct.

#[derive(Copy, Clone)]
pub struct Byte {
    inner: u8,
}

impl Byte {
    pub fn inner(&self) -> u8 {
        self.inner
    }
}

impl fathom_runtime::Format for Byte {
    type Host = Byte;
}

impl<'data> fathom_runtime::ReadFormat<'data> for Byte {
    fn read(reader: &mut fathom_runtime::FormatReader<'data>) -> Result<Byte, fathom_runtime::ReadError> {
        let inner = reader.read::<fathom_runtime::U8>()?;

        Ok(Byte {
            inner,
        })
    }
}
