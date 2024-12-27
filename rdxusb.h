#pragma once
#include <stdint.h>

/** Extended (full 29-bit) frame. This is set on practically all FRC-related messages. */
#define RDXUSB_MESSAGE_FLAG_EXT 0x80000000
/** RTR frame */
#define RDXUSB_MESSAGE_FLAG_RTR 0x40000000
/** Error frame */
#define RDXUSB_MESSAGE_FLAG_ERR 0x20000000

#ifdef _MSC_VER
#pragma pack(push, 4)
/** Packet that is sent and received from rdxusb APIs.  */
struct rdxusb_packet {
#else
/** Packet that is sent and received from rdxusb APIs.  */
struct __attribute__((packed, aligned(4))) rdxusb_packet {
#endif
    /** Timestamp since device power-on (nanoseconds) */
    uint64_t timestamp_ns;
    /** CAN arbitration id. */
    uint32_t arb_id;
    /** Data length code. */
    uint8_t dlc;
    /** Channel associated with the packet. Zero most of the time. */
    uint8_t channel;
    /** Misc flags (unused for now) */
    uint16_t flags;
    /** 
     * data (max size: 64 bytes) 
     * USB-FS devices (e.g. original canandgyro/canandcolor) only support data up to the first 48 bytes.
     */
    uint8_t data[64];
};
#ifdef _MSC_VER
#pragma pack(pop)
#endif

/** Represents a device visible to rdxusb. */
struct rdxusb_device_entry {
    /** Null-terminated serial number string. */
    char serial[256];
    /** Null-terminated manufacturer string. */
    char manufacturer[256];
    /** Null-terminated product string. */
    char product_string[256];
    /** USB vendor id */
    uint16_t vid;
    /** USB product id */
    uint16_t pid;
    /** Bus number of the device */
    uint8_t bus_number;
    /** Device address of the device */
    uint8_t device_address;
};

typedef uint64_t rdxusb_iter_id;

#ifdef __cplusplus
extern "C" {
#endif 

/**
 * Directs rdxusb to open a device with the associated vid/pid/serial number tuple.
 * 
 * rdxusb will spawn an event loop that will continually attempt to open a matching device and
 * send/receive messages from it. If connection with the matching device is lost, reconnection is 
 * continually attempted.
 * 
 * @param vid USB vendor ID to match
 * @param pid USB product ID to match
 * @param serial_number an optional serial number string. This MUST be utf-8 or NULL.
 * @param close_on_dc if true, closes the device handle on device disconnect
 * @return a non-negative device handle on success, negative on error
 */
int32_t rdxusb_open_device(uint16_t vid, uint16_t pid, const char* serial_number, bool close_on_dc);

/**
 * Forces the RdxUsb event loop to rescan USB devices.
 * @return 0 on success, negative on error
 */
int32_t rdxusb_force_scan_devices(void);

/**
 * Reads packets into the specified buffer.
 * 
 * @param handle_id a handle id returned from rdxusb_open_device
 * @param channel the USB channel to read from.
 *                The number of channels a device has is device dependent, but for now just pass in 0.
 * @param packets a pointer to the packet buffer to read into. Must not be NULL.
 * @param max_packets the maximum number of packets to read into the packet buffer.
 * @param packets_read pointer updated with how many packets were actually read. Must not be NULL.
 * @return 0 on success, negative on error
 */
int32_t rdxusb_read_packets(int32_t handle_id, uint8_t channel, 
                            struct rdxusb_packet* packets, 
                            uint64_t max_packets, uint64_t* packets_read);

/**
 * Writes packets from the specified buffer.
 * 
 * @param handle_id a handle id returned from rdxusb_open_device
 * @param packets a pointer to the packet buffer to write from. Must not be NULL.
 * @param packets_len the number of packets to write from the packet buffer.
 * @param packets_written pointer updated with how many packets were actually written. Must not be NULL.
 * @return 0 on success, negative on error
 */
int32_t rdxusb_write_packets(int32_t handle_id, struct rdxusb_packet* packets, 
                            uint64_t packets_len, uint64_t* packets_written);

/**
 * Closes the specified device, and stops reading from it.
 * 
 * If the handle ID is already closed or invalid, this returns 0.
 * 
 * @param handle_id a handle id returned from rdxusb_open_device
 * @return 0 on success, negative on error.
 */
int32_t rdxusb_close_device(int32_t handle_id);

/**
 * Creates a new USB device iterator.
 * 
 * @param iter_id pointer where the iterator handle will be written
 * @param n_devices the number of USB devices available to the iterator
 * @return 0 on success, negative on error
 */
int32_t rdxusb_new_device_iterator(rdxusb_iter_id* iter_id, uint64_t* n_devices);

/**
 * Gets a device by index in an iterator.
 * 
 * @param iter_id iterator handle to pull from
 * @param device_idx index to pull from. Must be 0 <= device_idx < n_devices.
 * @param device_entry pointer to write the USB device entry into. Must not be NULL.
 * @return 0 on success, negative on error
 */
int32_t rdxusb_get_device_in_iterator(rdxusb_iter_id iter_id, uint64_t device_idx,
                                      struct rdxusb_device_entry* device_entry);

/**
 * Frees a device iterator.
 * 
 * @param iter_id iterator to free
 * @return 0 on success, negative on error
 */
int32_t rdxusb_free_device_iterator(rdxusb_iter_id iter_id);

#ifdef __cplusplus
}
#endif