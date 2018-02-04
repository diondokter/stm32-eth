#![no_std]
#![feature(used)]
//#![feature(core_intrinsics)]
//#![feature(lang_items)]
#![feature(alloc, global_allocator, allocator_api, box_heap)]

extern crate cortex_m;
extern crate cortex_m_rt;
extern crate cortex_m_semihosting;
#[macro_use(exception, interrupt)]
extern crate stm32f429x;
extern crate alloc_cortex_m;
extern crate alloc;
extern crate volatile_register;

use cortex_m::asm;
use stm32f429x::{Interrupt, Peripherals, CorePeripherals, NVIC};

use core::fmt::Write;
use cortex_m_semihosting::hio;

mod init_alloc;
pub use init_alloc::ALLOCATOR;
mod eth;
use eth::Eth;

fn main() {
    let heap_size = init_alloc::init();
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "Heap: {} bytes", heap_size).unwrap();

    let p = Peripherals::take()
        .expect("Peripherals");
    let mut cp = CorePeripherals::take()
        .expect("CorePeripherals");

    writeln!(stdout, "Enabling ethernet...").unwrap();

    let mut eth = cortex_m::interrupt::free(|cs| {
        let mut eth = Eth::new(p.ETHERNET_MAC, p.ETHERNET_DMA);
        eth.init_pins(cs, &p.RCC, &p.GPIOA, &p.GPIOB, &p.GPIOC, &p.GPIOG);
        eth.init(cs, &p.RCC, &p.SYSCFG, &mut cp.NVIC);
        eth.start_rx(8);
        // eth.start_rx(1);
        eth
    });

    while ! eth.status().link_detected() {
        writeln!(stdout, "Ethernet: waiting for link").unwrap();
    }
    let status = eth.status();
    writeln!(
        stdout,
        "Ethernet: link detected with {} Mbps/{}",
        status.speed(),
        match status.is_full_duplex() {
            Some(true) => "FD",
            Some(false) => "HD",
            None => "?",
        }
    ).unwrap();

    let mut rx_bytes = 0usize;
    let mut rx_pkts = 0usize;
    loop {
        // asm::wfi();
        // writeln!(stdout, "I").unwrap();
        match eth.recv_next() {
            None => asm::wfi(),
            Some(pkt) => {
                // write!(stdout, "[Rx] {} bytes:", pkt.len());
                // for i in 0..pkt.len() {
                //     write!(stdout, " {:02X}", pkt[i]);
                // }
                // writeln!(stdout, "");
                rx_bytes += pkt.len();
                rx_pkts += 1;
                if rx_pkts % 10000 == 0 {
                    writeln!(stdout, "Received {} ({} KB)", rx_pkts, rx_bytes / 1024).unwrap();
                }
            },
        }
    }
}

fn eth_interrupt_handler() {
    let p = unsafe { Peripherals::steal() };

    // Clear interrupt flags
    p.ETHERNET_DMA.dmasr.write(|w|
        w
        .nis().set_bit()
        .rs().set_bit()
    );
}

#[used]
interrupt!(ETH, eth_interrupt_handler);
