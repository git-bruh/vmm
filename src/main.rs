use kvm_bindings::{KVM_EXIT_DEBUG, KVM_EXIT_HLT, KVM_EXIT_IO};
use nix::sys::{mman, mman::MapFlags, mman::ProtFlags};
use std::{env, fs::File, io::Read, num::NonZeroUsize, os::fd::BorrowedFd, slice};
use vmm::{
    bootparam::boot_e820_entry, kvm::Kvm, linux_loader::BzImage, util, util::WrappedAutoFree,
};

const MAPPING_SIZE: usize = 1 << 30;

const CMDLINE: &[u8] = b"console=ttyS0 earlyprintk=ttyS0 rdinit=/init\0";

const ADDR_BOOT_PARAMS: usize = 0x10000;
const ADDR_CMDLINE: usize = 0x20000;
const ADDR_KERNEL32: usize = 0x100000;
const ADDR_INITRAMFS: usize = 0xf000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kvm = Kvm::new()?;
    let mut bz_image = Vec::new();

    File::open(env::args().nth(1).expect("no bzImage passed!"))
        .expect("failed to open bzImage!")
        .read_to_end(&mut bz_image)
        .expect("failed to read!");

    let mut initramfs = Vec::new();

    File::open(env::args().nth(2).expect("no initramfs passed!"))
        .expect("failed to open initramfs")
        .read_to_end(&mut initramfs)
        .expect("failed to read!");

    let loader = BzImage::new(
        &bz_image,
        ADDR_CMDLINE.try_into().expect("cmdline address too large!"),
        Some(
            ADDR_INITRAMFS
                .try_into()
                .expect("initramfs address too large!"),
        ),
        Some(initramfs.len().try_into().expect("initramfs too big")),
        &[
            // Memory before the EBDA entry
            boot_e820_entry {
                addr: 0,
                size: 0x9fc00,
                // E820_RAM
                type_: 1,
            },
            // Reserved EBDA entry
            boot_e820_entry {
                addr: 0x9fc00,
                size: 1 << 10,
                // E820_RESERVED,
                type_: 2,
            },
            // Memory after the beginning of the kernel image
            boot_e820_entry {
                addr: 0x100000,
                size: MAPPING_SIZE as u64 - 0x100000,
                type_: 1,
            },
        ],
    )
    .expect("failed to construct loader!");

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

    let mapped_slice = unsafe { slice::from_raw_parts_mut(*wrapped_mapping as _, MAPPING_SIZE) };

    unsafe {
        std::ptr::copy_nonoverlapping(
            &loader.boot_params(),
            wrapped_mapping.add(ADDR_BOOT_PARAMS) as *mut _,
            1,
        );
        let kernel32 = loader.kernel32_slice();
        std::ptr::copy_nonoverlapping(
            kernel32.as_ptr(),
            wrapped_mapping.add(ADDR_KERNEL32) as *mut _,
            kernel32.len(),
        );
        std::ptr::copy_nonoverlapping(
            CMDLINE.as_ptr(),
            wrapped_mapping.add(ADDR_CMDLINE) as *mut _,
            CMDLINE.len(),
        );
        std::ptr::copy_nonoverlapping(
            initramfs.as_ptr(),
            wrapped_mapping.add(ADDR_INITRAMFS) as *mut _,
            initramfs.len(),
        );
    }

    util::setup_gdt(mapped_slice);
    util::setup_paging(mapped_slice);

    kvm.set_user_memory_region(0x0, MAPPING_SIZE as u64, *wrapped_mapping as u64)?;
    kvm.set_vcpu_regs(&util::setup_regs(
        // 64-bit code is located 512 bytes ahead of the 32-bit code
        ADDR_KERNEL32 as u64 + 0x200,
        // boot params are stored in rsi
        ADDR_BOOT_PARAMS as u64,
    ))?;
    kvm.set_vcpu_sregs(&util::setup_sregs())?;
    kvm.set_tss_addr(0xFFFFD000)?;
    kvm.setup_cpuid()?;

    let mut buffer = String::new();

    loop {
        let kvm_run = kvm.run()?;

        unsafe {
            match (*kvm_run).exit_reason {
                KVM_EXIT_HLT => {
                    eprintln!("KVM_EXIT_HLT");
                    break;
                }
                KVM_EXIT_DEBUG => {
                    eprintln!(
                        "{:#?}\n{:#?}",
                        kvm.get_vcpu_regs(),
                        (*kvm_run).__bindgen_anon_1.debug
                    );
                }
                // TODO abstract out this struct so we don't have to write hacky
                // C-style code here
                KVM_EXIT_IO => {
                    let port = (*kvm_run).__bindgen_anon_1.io.port;
                    let byte = *((kvm_run as u64 + (*kvm_run).__bindgen_anon_1.io.data_offset)
                        as *const u8);

                    if port == 0x3f8 {
                        match byte {
                            b'\r' | b'\n' => {
                                println!("{buffer}");
                                buffer.clear();
                            }
                            c => {
                                buffer.push(c as char);
                            }
                        }
                    }

                    eprintln!("IO for port {port}: {byte:#X}");

                    // `in` instruction, tell it that we're ready to receive data (XMTRDY)
                    // arch/x86/boot/tty.c
                    if (*kvm_run).__bindgen_anon_1.io.direction == 0 {
                        *((kvm_run as *mut u8)
                            .add((*kvm_run).__bindgen_anon_1.io.data_offset as usize)) = 0x20;
                    }
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
