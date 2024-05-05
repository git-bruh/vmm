use crate::{
    bootparam::{boot_e820_entry, boot_params, CAN_USE_HEAP, LOADED_HIGH},
    constants::SegmentFlags,
};
use kvm_bindings::kvm_segment;
use std::{mem, ptr};

pub struct BzImage<'a> {
    bz_image: &'a [u8],
    boot_params: boot_params,
}

#[derive(Debug)]
pub enum LoaderError {
    /// Image is not large enough relative to what it's metadata claims
    ImageTooSmall,
    /// Corrupt image, mismatched magic bytes / other metadata
    InvalidImage,
    /// Too many E820 entries
    TooManyEntries,
}

/// CS, placed at 0x10
/// See `pack_segment` for more details
pub const CODE_SEGMENT: kvm_segment = kvm_segment {
    base: 0,
    limit: 0xFFFFFFFF,
    selector: 0x10,
    type_: SegmentFlags::CODE_SEGMENT | SegmentFlags::CODE_READ,
    present: 1,
    dpl: 0,
    db: 0,
    s: 1,
    l: 1,
    g: 1,
    avl: 0,
    unusable: 0,
    padding: 0,
};

/// DS, placed at 0x18
/// See `pack_segment` for more details
pub const DATA_SEGMENT: kvm_segment = kvm_segment {
    base: 0,
    limit: 0xFFFFFFFF,
    selector: 0x18,
    type_: SegmentFlags::DATA_WRITE,
    present: 1,
    dpl: 0,
    db: 1,
    s: 1,
    l: 0,
    g: 1,
    avl: 0,
    unusable: 0,
    padding: 0,
};

/// Start offset of the 32-bit (non-real-mode) kernel
fn kernel_byte_offset(boot_params: &boot_params) -> usize {
    (match boot_params.hdr.setup_sects as usize {
        0 => 4,
        sects => sects,
    } + 1)
        * 512
}

impl<'a> BzImage<'a> {
    pub fn new(
        bz_image: &'a [u8],
        cmdline_addr: u32,
        initramfs_addr: Option<u32>,
        initramfs_size: Option<u32>,
        e820_entries: &[boot_e820_entry],
    ) -> Result<BzImage<'a>, LoaderError> {
        // The setup_header is located at offset 0x1f1 (`hdr` field) from the start
        // of `boot_params` (which is also the start of the kernel image)
        let mut boot_params = boot_params::default();

        // Ref: 1.3. Details of Header Fields
        // We just need to modify a few fields here to tell the kernel about
        // the environment we're setting up. Rest of the information is already
        // filled in the struct (embedded in the bz_image)

        if bz_image.len() < mem::size_of_val(&boot_params) {
            return Err(LoaderError::ImageTooSmall);
        }

        unsafe {
            ptr::copy_nonoverlapping(bz_image.as_ptr().cast(), &mut boot_params, 1);
        }

        // `boot_flag` and `header` are magic values documented in the boot protocol
        // > Then, the setup header at offset 0x01f1 of kernel image on should be
        // > loaded into struct boot_params and examined. The end of setup header
        // > can be calculated as follows: 0x0202 + byte value at offset 0x0201
        // 0x0201 refers to the 16 bit `jump` field of the `setup_header` struct
        // Contains an x86 jump instruction, 0xEB followed by a signed offset relative to byte 0x202
        // So we just read a byte out of it, i.e. the offset from the header (0x0202)
        // It should always be 106 unless a field after `kernel_info_offset` is added
        if boot_params.hdr.boot_flag != 0xAA55
            || boot_params.hdr.header != 0x53726448
            || (boot_params.hdr.jump >> 8) != 106
        {
            return Err(LoaderError::InvalidImage);
        }

        if bz_image.len() < kernel_byte_offset(&boot_params) {
            return Err(LoaderError::ImageTooSmall);
        }

        // VGA display
        boot_params.hdr.vid_mode = 0xFFFF;

        // "Undefined" Bootloader ID
        boot_params.hdr.type_of_loader = 0xFF;

        // LOADED_HIGH: the protected-mode code is loaded at 0x100000
        // CAN_USE_HEAP: Self explanatory
        boot_params.hdr.loadflags |= (LOADED_HIGH | CAN_USE_HEAP) as u8;

        boot_params.hdr.ramdisk_image = initramfs_addr.unwrap_or(0);
        boot_params.hdr.ramdisk_size = initramfs_size.unwrap_or(0);

        // https://www.kernel.org/doc/html/latest/arch/x86/boot.html#sample-boot-configuration
        // 0xe000 - 0x200
        boot_params.hdr.heap_end_ptr = 0xde00;
        // The command line parameters can be located anywhere in 64-bit mode
        // Must be NUL terminated
        boot_params.hdr.cmd_line_ptr = cmdline_addr;
        boot_params.ext_cmd_line_ptr = 0;

        boot_params.e820_entries = e820_entries
            .len()
            .try_into()
            .map_err(|_| LoaderError::TooManyEntries)?;
        boot_params.e820_table[..e820_entries.len()].copy_from_slice(e820_entries);

        Ok(Self {
            bz_image,
            boot_params,
        })
    }

    /// Get the boot parameters
    pub fn boot_params(&self) -> boot_params {
        self.boot_params
    }

    /// Get a slice to the image, pointing to the 32-bit startup code
    pub fn kernel32_slice(&self) -> &'a [u8] {
        &self.bz_image[kernel_byte_offset(&self.boot_params)..]
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        linux_loader::{CODE_SEGMENT, DATA_SEGMENT},
        util::pack_segment,
    };

    #[test]
    fn pack_cs() {
        assert_eq!(
            pack_segment(&CODE_SEGMENT),
            0b10101111100110100000000000000000000000001111111111111111
        );
    }

    #[test]
    fn pack_ds() {
        assert_eq!(
            pack_segment(&DATA_SEGMENT),
            0b11001111100100100000000000000000000000001111111111111111
        );
    }
}
