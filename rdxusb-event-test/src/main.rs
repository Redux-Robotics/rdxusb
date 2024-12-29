use std::time::Duration;

use rdxusb::RdxUsbPacket;

fn main() {
    let handle = rdxusb::c_api::rdxusb_open_device(0x16d0, 0x1278, c"00-0-0000-000-0-0".as_ptr(), false, 48);
    if handle < 0 {
        panic!("could not open device: {handle}");
    }

    ctrlc::set_handler(move || {
        println!("ctrl-c detected, halting");
        rdxusb::c_api::rdxusb_close_device(handle);
        std::process::exit(0);
    }).ok();

    let mut i = 0u64;
    loop {
        let mut packets: Vec<RdxUsbPacket> = Vec::with_capacity(48);
        let mut packets_read = 0u64;

        let result = rdxusb::c_api::rdxusb_read_packets(handle, 0, packets.as_mut_ptr(), 32, &mut packets_read);
        println!("i: {i} Status {result} Read {packets_read} packets");

        i += 1;
        std::thread::sleep(Duration::from_millis(100));
    }
}
