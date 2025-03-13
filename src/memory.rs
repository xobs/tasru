//! Memory traits can be used to read or write data in a given target.
//! This data may be live (for example communicating with a target via
//! a debugger or an emulator), or may be at-rest (for example querying
//! an .ihex image of a running device).

/// A device that can read memory addresses. This may be a live device,
/// a core dump, or some other operation.
pub trait Read {
    type Error: core::error::Error;

    /// Read one 8-bit value from the specified address.
    fn read_u8(&mut self, address: u64) -> Result<u8, Self::Error>;

    /// Read one 16-bit value from the specified address. The address does
    /// not need to be aligned, but performance may be improved if it is.
    fn read_u16(&mut self, address: u64) -> Result<u16, Self::Error> {
        Ok(u16::from_le_bytes([
            self.read_u8(address)?,
            self.read_u8(address + 1)?,
        ]))
    }

    /// Read one 32-bit value from the specified address. The address does
    /// not need to be aligned, but performance may be improved if it is.
    fn read_u32(&mut self, address: u64) -> Result<u32, Self::Error> {
        Ok(u32::from_le_bytes([
            self.read_u8(address)?,
            self.read_u8(address + 1)?,
            self.read_u8(address + 2)?,
            self.read_u8(address + 3)?,
        ]))
    }

    /// Read one 64-bit value from the specified address. The address does
    /// not need to be aligned, but performance may be improved if it is.
    fn read_u64(&mut self, address: u64) -> Result<u64, Self::Error> {
        Ok(u64::from_le_bytes([
            self.read_u8(address)?,
            self.read_u8(address + 1)?,
            self.read_u8(address + 2)?,
            self.read_u8(address + 3)?,
            self.read_u8(address + 4)?,
            self.read_u8(address + 5)?,
            self.read_u8(address + 6)?,
            self.read_u8(address + 7)?,
        ]))
    }

    /// Read data into the buffer. If an error occurs, then the buffer contents
    /// are undefined and may contain partial data.
    fn read(&mut self, data: &mut [u8], address: u64) -> Result<(), Self::Error> {
        for (offset, byte) in data.iter_mut().enumerate() {
            *byte = self.read_u8(address + offset as u64)?;
        }
        Ok(())
    }

    /// Indicates that a burst of data will be read. The source can use this
    /// information to buffer new contents from the target.
    fn begin(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Indicates the data access has finished.
    fn finish(&mut self) {}
}

/// Write data to the device. This is currently unused in tasru.
pub trait Write {
    type Error: core::error::Error;

    fn write_u8(&mut self, data: u8, address: u64) -> Result<(), Self::Error>;

    fn write_u16(&mut self, data: u16, address: u64) -> Result<(), Self::Error> {
        for (offset, data) in data.to_le_bytes().into_iter().enumerate() {
            self.write_u8(data, address + offset as u64)?;
        }
        Ok(())
    }

    fn write_u32(&mut self, data: u32, address: u64) -> Result<(), Self::Error> {
        for (offset, data) in data.to_le_bytes().into_iter().enumerate() {
            self.write_u8(data, address + offset as u64)?;
        }
        Ok(())
    }

    fn write_u64(&mut self, data: u64, address: u64) -> Result<(), Self::Error> {
        for (offset, data) in data.to_le_bytes().into_iter().enumerate() {
            self.write_u8(data, address + offset as u64)?;
        }
        Ok(())
    }

    fn write(&mut self, data: &[u8], address: u64) -> Result<(), Self::Error> {
        for (offset, byte) in data.iter().enumerate() {
            self.write_u8(*byte, address + offset as u64)?
        }
        Ok(())
    }

    /// Indicates that a burst of data will be read. The source can use this
    /// information to buffer new contents from the target.
    fn begin(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Indicates the data access has finished.
    fn finish(&mut self) {}
}

pub trait ReadWrite: Read + Write {
    type Error: core::error::Error;
}
