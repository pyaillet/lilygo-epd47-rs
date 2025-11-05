use esp_hal::{
    gpio::Level,
    peripherals,
    rmt,
    rmt::{Channel, PulseCode, Tx, TxChannelCreator},
    time::Rate,
    Blocking,
};

pub(crate) struct Rmt<'a> {
    tx_channel: Option<Channel<'a, Blocking, Tx>>,
    _rmt: peripherals::RMT<'a>,
}

impl<'a> Rmt<'a> {
    pub(crate) fn new(_rmt: peripherals::RMT<'a>) -> Self {
        Rmt {
            tx_channel: None,
            _rmt,
        }
    }

    fn ensure_channel(&mut self) -> Result<(), crate::Error> {
        if self.tx_channel.is_some() {
            return Ok(());
        }
        let freq = Rate::from_mhz(80);
        let rmt = rmt::Rmt::new(
            unsafe { peripherals::RMT::steal() }, // TODO: find better solution
            freq,
        )
        .map_err(crate::Error::Rmt)?;
        let config = rmt::TxChannelConfig::default()
            .with_clk_divider(8)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true)
            .with_carrier_modulation(false)
            .with_carrier_level(Level::Low);
        let tx_channel = rmt
            .channel1
            .configure_tx(
                unsafe { peripherals::GPIO38::steal() }, // TODO: find better solution
                config,
            )
            .map_err(crate::Error::Rmt)?;
        self.tx_channel = Some(tx_channel);
        Ok(())
    }

    pub(crate) fn pulse(&mut self, high: u16, low: u16, wait: bool) -> Result<(), crate::Error> {
        self.ensure_channel()?;
        let tx_channel = self.tx_channel.take().ok_or(crate::Error::Unknown)?;
        let data = if high > 0 {
            [
                PulseCode::new(Level::High, high, Level::Low, low),
                PulseCode::end_marker(),
            ]
        } else {
            [
                PulseCode::new(Level::Low, low, Level::Low, 0),
                PulseCode::end_marker(),
            ]
        };
        let tx = tx_channel.transmit(&data).map_err(crate::Error::Rmt)?;
        // FIXME: This is the culprit.. We need the channel later again but can't wait
        // due to some time sensitive operations. Not sure how to solve this
        if wait {
            self.tx_channel = Some(
                tx.wait()
                    .map_err(|(err, _)| err)
                    .map_err(crate::Error::Rmt)?,
            );
        }
        Ok(())
    }
}
