use ghostpath_core::Result;

pub struct TunDevice {
    name: String,
    mtu: u16,
}

impl TunDevice {
    pub fn new(name: impl Into<String>, mtu: u16) -> Self {
        Self {
            name: name.into(),
            mtu,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub async fn open(&mut self) -> Result<()> {
        tracing::info!(name = %self.name, mtu = self.mtu, "Initializing TUN interface");
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        tracing::info!(name = %self.name, "Closing TUN interface");
        Ok(())
    }
}
