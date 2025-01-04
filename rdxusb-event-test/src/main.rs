use std::time::Duration;

use rdxusb::{RdxUsbPacket, MESSAGE_ARB_ID_DEVICE, MESSAGE_ARB_ID_EXT};


fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("trace"));
    let handle = rdxusb::c_api::rdxusb_open_device(0x16d0, 0x1279, c"04-0-0000-000-E-1".as_ptr(), false, 48);
    if handle < 0 {
        panic!("could not open device: {handle}");
    }

    ctrlc::set_handler(move || {
        println!("ctrl-c detected, halting");
        rdxusb::c_api::rdxusb_close_device(handle);
        std::process::exit(0);
    }).ok();

    let mut data = [0u8; 64];
    data[0] = 1;
    let packet = RdxUsbPacket {
        timestamp_ns: 0,
        //  party mode
        arb_id: (15 | (7 << 6) | (0xe0000) | (0x6 << 24)) | MESSAGE_ARB_ID_EXT | MESSAGE_ARB_ID_DEVICE,
        dlc: 1,
        channel: 0,
        flags: 0,
        data: data,
    };

    // opening a handle isn't instantaneous. 
    std::thread::sleep(Duration::from_millis(100));
    let mut packets_written = 0u64;
    let result = rdxusb::c_api::rdxusb_write_packets(handle, &packet, 1, &mut packets_written);
    println!("write packet: {result} for {packets_written}");

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
