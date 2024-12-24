use canandmessage::canandgyro;
use rdxusb::host::RdxUsbFsHost;
use rdxusb_protocol::{RdxUsbFsPacket, MESSAGE_FLAG_EFF};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let device = nusb::list_devices().expect("can't init nusb")
    .find(|dev| (dev.vendor_id(), dev.product_id()) == (0x16d0, 0x1278))
    .expect("canandgyro not connected");

    println!("Device found: {:?}", device);

    let (mut dev, mut channels) = RdxUsbFsHost::open_device(device, 16).await?;
    println!("Device opened!!");

    println!("Device config: {:?}", dev.get_device_config().await?);
    let chan0 = &mut channels[0];

    tokio::spawn(async move {
        dev.poll(32, false).await.unwrap();
    });

    loop {
        let frame = chan0.read().await?;

        match TryInto::<canandgyro::Message>::try_into(canandmessage::CanandMessageWrapper(RdxUsbFsPacketW(frame.clone()))) {
            Ok(d) => println!("Found frame: {d:?}"),
            Err(_) => {
                println!("Found frame: {frame:?}")
            }
        };

        //println!("Found frame: {frame:?}");
        //let mut mutex = state.lock().await;
        //mutex.update(frame);
    }
}

pub struct RdxUsbFsPacketW(pub RdxUsbFsPacket);

impl canandmessage::CanandMessage<RdxUsbFsPacketW> for RdxUsbFsPacketW {
    fn get_data(&self) -> &[u8] {
        &self.0.data[..self.0.dlc as usize]
    }

    fn get_len(&self) -> u8 {
        self.0.dlc
    }

    fn get_id(&self) -> u32 {
        self.0.arb_id & 0x1fff_ffff
    }

    fn try_from_data(id: u32, data: &[u8]) -> Result<RdxUsbFsPacketW, canandmessage::CanandMessageError> {
        if data.len() > 48 { return Err(canandmessage::CanandMessageError::DataTooLarge); }
        let mut payload = [0u8; 48];
        payload[..data.len()].copy_from_slice(data);
        Ok(RdxUsbFsPacketW(RdxUsbFsPacket{ timestamp_ns: 0, arb_id: id | MESSAGE_FLAG_EFF, dlc: data.len() as u8, channel: 0, flags: 0, data: payload }))
    }
}