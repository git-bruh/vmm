use core::num::NonZeroUsize;
use kvm_bindings::{kvm_regs, kvm_segment, KVM_EXIT_HLT, KVM_EXIT_IO};
use nix::sys::{mman, mman::MapFlags, mman::ProtFlags};
use std::os::fd::BorrowedFd;
use vmm::kvm::Kvm;
use vmm::util::WrappedAutoFree;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
