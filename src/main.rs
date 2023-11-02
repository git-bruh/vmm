use core::num::NonZeroUsize;
use kvm_bindings::{kvm_regs, KVM_EXIT_HLT, KVM_EXIT_IO};
use nix::sys::{mman, mman::MapFlags, mman::ProtFlags};
use std::os::fd::BorrowedFd;
use vmm::kvm::Kvm;
use vmm::util::WrappedAutoFree;

const CODE: [u8; 12] = [
    0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
    0x00, 0xd8, /* add %bl, %al */
    0x04, b'0', /* add $'0', %al */
    0xee, /* out %al, (%dx) */
    0xb0, b'\n', /* mov $'\n', %al */
    0xee,  /* out %al, (%dx) */
    0xf4,  /* hlt */
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kvm = Kvm::new()?;

    let wrapped_mapping = WrappedAutoFree::new(
        unsafe {
            mman::mmap(
                None,
                NonZeroUsize::new(4096).expect("unreachable, passed > 0"),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_ANONYMOUS | MapFlags::MAP_SHARED,
                None::<BorrowedFd>,
                0,
            )?
        },
        |map| unsafe {
            mman::munmap(*map, 4096).expect("failed to unmap user memory region!");
        },
    );

    unsafe { std::ptr::copy_nonoverlapping(CODE.as_ptr(), wrapped_mapping.val as _, CODE.len()) }

    kvm.set_user_memory_region(4096, 4096, wrapped_mapping.val as u64)?;

    let mut sregs = kvm.get_vcpu_sregs()?;

    sregs.cs.base = 0;
    sregs.cs.selector = 0;

    kvm.set_vcpu_sregs(&sregs)?;

    let mut regs = kvm_regs::default();

    regs.rflags = 0x2;
    regs.rip = 4096;
    regs.rax = 4;
    regs.rbx = 2;

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
                        "IO for port {}: chr({})",
                        // TODO abstract out epic bindgen union moment
                        (*kvm_run).__bindgen_anon_1.io.port,
                        // TODO abstract out epic struct as bytes moment
                        *((kvm_run as u64 + (*kvm_run).__bindgen_anon_1.io.data_offset)
                            as *const u8)
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
