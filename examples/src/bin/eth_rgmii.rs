//! This example demonstrates how to configure the internal EMAC of the esp32s31 in RGMII mode
//! (10/100/1000 Mbps), like the ESP-IDF `ethernet/basic` example
//!
//! It uses the ESP-IDF default pins of the ESP32-S31 Functional-Core Board, whose PHY is a
//! YT8531 driven by the Generic IEEE 802.3 PHY driver. The YT8531 specific setup (autonegotiation
//! and RGMII clock delays) is done in `yt8531_rgmii_fixup`; adjust or remove it for another PHY.

#![allow(unknown_lints)]
#![allow(unexpected_cfgs)]

#[cfg(all(esp32s31, esp_idf_eth_use_esp32_emac))]
use esp_idf_svc::{
    eth::{BlockingEth, EmacConfig, EspEth, EthDriver, RgmiiEth, RgmiiPins},
    eventloop::EspSystemEventLoop,
    hal::peripherals::Peripherals,
    log::EspLogger,
    sys::EspError,
};
#[cfg(all(esp32s31, esp_idf_eth_use_esp32_emac))]
use log::info;

#[cfg(all(esp32s31, esp_idf_eth_use_esp32_emac))]
fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    let sys_loop = EspSystemEventLoop::take()?;

    // The RGMII pins are fixed to GPIO8..=19 on the esp32s31
    let rgmii = RgmiiPins {
        gpio8: pins.gpio8,
        gpio9: pins.gpio9,
        gpio10: pins.gpio10,
        gpio11: pins.gpio11,
        gpio12: pins.gpio12,
        gpio13: pins.gpio13,
        gpio14: pins.gpio14,
        gpio15: pins.gpio15,
        gpio16: pins.gpio16,
        gpio17: pins.gpio17,
        gpio18: pins.gpio18,
        gpio19: pins.gpio19,
    };

    let mut eth_driver = EthDriver::new_rgmii(
        peripherals.mac,
        pins.gpio5,
        pins.gpio6,
        rgmii,
        // No 50 MHz reference clock output to the PHY on GPIO35 (the ESP-IDF default)
        None,
        Some(pins.gpio7),
        // Auto-detect the PHY address
        None,
        EmacConfig::default(),
        sys_loop.clone(),
    )?;

    // The PHY specific setup has to be done after the driver is created and before it is started
    yt8531_rgmii_fixup(&mut eth_driver)?;

    let eth = EspEth::wrap(eth_driver)?;

    info!("Eth created");

    let mut eth = BlockingEth::wrap(eth, sys_loop.clone())?;

    info!("Starting eth...");

    eth.start()?;

    info!("Waiting for DHCP lease...");

    eth.wait_netif_up()?;

    let ip_info = eth.eth().netif().get_ip_info()?;

    info!("Eth DHCP info: {ip_info:?}");

    info!("Waiting 10 seconds before exiting...");

    std::thread::sleep(core::time::Duration::from_secs(10));

    Ok(())
}

/// YT8531 specific setup, not done by the Generic IEEE 802.3 PHY driver
#[cfg(all(esp32s31, esp_idf_eth_use_esp32_emac))]
fn yt8531_rgmii_fixup(driver: &mut EthDriver<'_, RgmiiEth>) -> Result<(), EspError> {
    // Extended register address / data registers
    const EXT_ADDR: u32 = 0x1E;
    const EXT_DATA: u32 = 0x1F;
    // Extended registers
    const EXT_CHIP_CONFIG: u32 = 0xA001;
    const EXT_RGMII_CONFIG1: u32 = 0xA003;

    // The YT8531 disables autonegotiation when it is reset by the Generic PHY driver
    // (undocumented, but observed), so enable it again
    driver.set_autonego(true)?;

    // RX: enable the ~2 ns coarse RX clock delay (`rxc_dly_en`, bit 8)
    driver.write_phy_reg(EXT_ADDR, EXT_CHIP_CONFIG)?;
    let value = driver.read_phy_reg(EXT_DATA)?;
    driver.write_phy_reg(EXT_DATA, value | (1 << 8))?;

    // TX: set `tx_delay_sel` (bits 3:0) and `tx_delay_sel_fe` (bits 7:4) to 13 steps x 150 ps (~1.95 ns)
    driver.write_phy_reg(EXT_ADDR, EXT_RGMII_CONFIG1)?;
    let value = driver.read_phy_reg(EXT_DATA)?;
    driver.write_phy_reg(EXT_DATA, (value & !0xFF) | (13 << 4) | 13)?;

    info!("YT8531 RGMII delays configured: RX ~2 ns (coarse), TX ~2 ns (13 x 150 ps)");

    Ok(())
}

#[cfg(not(all(esp32s31, esp_idf_eth_use_esp32_emac)))]
fn main() {
    use esp_idf_svc::{self as _};

    panic!("This example is configured for the esp32s31 (internal EMAC in RGMII mode), please adjust it to your module");
}
