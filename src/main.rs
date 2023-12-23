use core::num::NonZeroUsize;
use kvm_bindings::{kvm_regs, kvm_segment, KVM_EXIT_HLT, KVM_EXIT_IO};
use nix::sys::{mman, mman::MapFlags, mman::ProtFlags};
use std::{io::Read, fs::File, os::fd::BorrowedFd};
use vmm::kvm::Kvm;
use vmm::util::WrappedAutoFree;
use vmm::bootparam::{boot_params, LOADED_HIGH, CAN_USE_HEAP};

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/", "write_serial"));
const MAPPING_SIZE: usize = 1 << 24;

/// Paging
#[allow(non_snake_case)]
mod PageFlags {
    /// The page is present in physical memory
    pub const PRESENT: u64 = 1 << 0;
    /// The page is read/write
    pub const READ_WRITE: u64 = 1 << 1;
    /// Make PDE map to a 4MiB page, Page Size Extension must be enabled
    pub const PAGE_SIZE: u64 = 1 << 7;
}

/// Control Register 0
#[allow(non_snake_case)]
mod Cr0Flags {
    /// Enable protected mode
    pub const PE: u64 = 1 << 0;
    /// Enable paging
    pub const PG: u64 = 1 << 31;
}

/// Control Register 4
#[allow(non_snake_case)]
mod Cr4Flags {
    /// Page Size Extension
    pub const PSE: u64 = 1 << 4;
    /// Physical Address Extension, size of large pages is reduced from
    /// 4MiB to 2MiB and PSE is enabled regardless of the PSE bit
    pub const PAE: u64 = 1 << 5;
}

/// Extended Feature Enable Register
#[allow(non_snake_case)]
mod EferFlags {
    /// Long Mode Enable
    pub const LME: u64 = 1 << 8;
    /// Long Mode Active
    pub const LMA: u64 = 1 << 10;
}

// Make a temporary bitwise copy that a reference can point to
// For use in macros like println
macro_rules! unaligned_read {
    ($x:expr) => {
        {
            let tmp = $x;
            tmp
        }
    };
}

fn load() {
    let mut kernel = Vec::new();

    File::open("bzImage")
        .unwrap()
        .read_to_end(&mut kernel)
        .unwrap();

    // The setup_header is located at offset 0x1f1 (`hdr` field) from the start
    // of `boot_params` (which is also the start of the kernel image)
    let boot_params = unsafe { &mut *(kernel.as_mut_ptr().cast::<boot_params>()) };

    // Ref: 1.3. Details of Header Fields
    // We just need to modify a few fields here to tell the kernel about
    // the environment we're setting up. Rest of the information is already
    // filled in the struct (embedded in the bzImage)

    // Some magic values we can check, just to be sure we're not
    // reading garbage :p
    assert_eq!(0xAA55, unaligned_read!(boot_params.hdr.boot_flag));
    assert_eq!(0x53726448, unaligned_read!(boot_params.hdr.header));

    println!("setup_sects: {}", boot_params.hdr.setup_sects);

    // VGA Display
    boot_params.hdr.vid_mode = 0xFFFF;

    // "Undefined" Bootloader ID
    boot_params.hdr.type_of_loader = 0xFF;

    // LOADED_HIGH: the protected-mode code is loaded at 0x100000
    // CAN_USE_HEAP: Self explanatory; heap_end_ptr is valid
    boot_params.hdr.loadflags |= (LOADED_HIGH | CAN_USE_HEAP) as u8;

    // No initramfs support
    boot_params.hdr.ramdisk_image = 0;
    boot_params.hdr.ramdisk_size = 0;

    // Assume protocol version >= 0x202
    boot_params.hdr.heap_end_ptr = 0xe000 - 0x200;
    boot_params.hdr.cmd_line_ptr = boot_params.hdr.heap_end_ptr as u32;

    // The 32-bit (non-real-mode) kernel starts at offset (setup_sects+1)*512
    // in the kernel file (again, if setup_sects == 0 the real value is 4.) 
    let kernel_offset = (match boot_params.hdr.setup_sects as u32 {
        0 => 4,
        sects => sects    
    } + 1) * 512;

    // Then, the setup header at offset 0x01f1 of kernel image on should be
    // loaded into struct boot_params and examined. The end of setup header
    // can be calculated as follows:
    //     0x0202 + byte value at offset 0x0201

    // 0x0201 refers to the 16 bit `jump` field of the `setup_header` struct
    // Contains an x86 jump instruction, 0xEB followed by a signed offset relative to byte 0x202
    // So we just read a byte out of it, i.e. the offset from the header
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    load();
    std::process::abort();

    let kvm = Kvm::new()?;

    // Create a mapping for the "user" memory region where we'll copy the
    // startup code into
    let wrapped_mapping = WrappedAutoFree::new(
        unsafe {
            mman::mmap(
                None,
                NonZeroUsize::new(MAPPING_SIZE).expect("unreachable, passed > 0"),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_ANONYMOUS | MapFlags::MAP_SHARED,
                None::<BorrowedFd>,
                0,
            )?
        },
        |map| unsafe {
            mman::munmap(map, MAPPING_SIZE).expect("failed to unmap user memory region!");
        },
    );

    unsafe { std::ptr::copy_nonoverlapping(CODE.as_ptr(), *wrapped_mapping as _, CODE.len()) }

    let pml4_offset = 0x1000;
    let pdpt_offset = 0x2000;
    let pd_offset = 0x3000;

    unsafe {
        *(wrapped_mapping.add(pml4_offset as usize) as *mut u64) =
            PageFlags::PRESENT | PageFlags::READ_WRITE | pdpt_offset;
        *(wrapped_mapping.add(pdpt_offset as usize) as *mut u64) =
            PageFlags::PRESENT | PageFlags::READ_WRITE | pd_offset;
        *(wrapped_mapping.add(pd_offset as usize) as *mut u64) =
            PageFlags::PRESENT | PageFlags::READ_WRITE | PageFlags::PAGE_SIZE;
    }

    // The mapping is placed at address 0 in the VM
    kvm.set_user_memory_region(0x0, MAPPING_SIZE as u64, *wrapped_mapping as u64)?;

    let mut sregs = kvm.get_vcpu_sregs()?;

    sregs.cr3 = pml4_offset;
    sregs.cr4 = Cr4Flags::PAE;
    sregs.cr0 = Cr0Flags::PE | Cr0Flags::PG;
    sregs.efer = EferFlags::LMA | EferFlags::LME;

    /// XXX explore why some other projects set bunch of unused flags here and
    /// store the segment in ds, es, fs, gs, and ss (cite IA64 manual)
    /// XXX explore what is _actually_ required wrt GDT, etc.
    let segment = kvm_segment {
        // Level 0 privilege
        dpl: 0,
        db: 0,
        // Long Mode
        l: 1,
        ..Default::default()
    };

    sregs.cs = segment;

    kvm.set_vcpu_sregs(&sregs)?;

    let mut regs = kvm_regs::default();

    // Specified by x86 (reserved bit)
    regs.rflags = 1 << 1;
    // Set the instruction pointer to the start of the copied code
    regs.rip = 0;
    regs.rsp = 0x200000;

    kvm.set_vcpu_regs(&regs)?;

    loop {
        let kvm_run = kvm.run()?;

        unsafe {
            match (*kvm_run).exit_reason {
                KVM_EXIT_HLT => {
                    eprintln!("KVM_EXIT_HLT");
                    break;
                }
                KVM_EXIT_IO => {
                    println!(
                        "IO for port {}: {}",
                        // TODO abstract out epic bindgen union moment
                        (*kvm_run).__bindgen_anon_1.io.port,
                        // TODO abstract out epic struct as bytes moment
                        *((kvm_run as u64 + (*kvm_run).__bindgen_anon_1.io.data_offset)
                            as *const u8) as char
                    );
                }
                reason => {
                    eprintln!("Unhandled exit reason: {reason}");
                    break;
                }
            }
        }
    }

    Ok(())
}
