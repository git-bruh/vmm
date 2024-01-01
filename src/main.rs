use core::num::NonZeroUsize;
use kvm_bindings::{kvm_dtable, kvm_regs, kvm_segment, KVM_EXIT_DEBUG, KVM_EXIT_HLT, KVM_EXIT_IO};
use nix::sys::{mman, mman::MapFlags, mman::ProtFlags};
use std::{fs::File, io::Read, os::fd::BorrowedFd};
use vmm::bootparam::{boot_e820_entry, boot_params, CAN_USE_HEAP, LOADED_HIGH};
use vmm::kvm::Kvm;
use vmm::util::WrappedAutoFree;

const E820_RAM: u32 = 1;

const CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/", "write_serial"));
const MAPPING_SIZE: usize = 1 << 30;

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

enum GdtSegType {
    Code,
    Data,
}

// Make a temporary bitwise copy that a reference can point to
// For use in macros like println
macro_rules! unaligned_read {
    ($x:expr) => {{
        let tmp = $x;
        tmp
    }};
}

// 3.4.5 Segment Descriptors
fn get_gdt_segment(kind: GdtSegType) -> u64 {
    // This is an overly verbose way of doing this, but should hopefully be
    // more useful than magic numbers found in other codebases

    // These flags must be conditionally set wrt the kind of segment
    let (segment_type, l, db) = match kind {
        // __BOOT_CS must have execute/read permission
        GdtSegType::Code => {
            (
                // Code segment is implied to be executable
                // 1st bit toggles read permissions, 3rd bit indicates a Code segment
                ((1 << 1) | (1 << 3)),
                // L, 64-bit mode
                (1 << 1),
                (0),
            )
        }
        // __BOOT_DS must have read/write permission
        GdtSegType::Data => {
            (
                // Read permission is implied
                // 1st bit toggles write permissions
                // No bit is required to indicate a data segment
                (1 << 1),
                (0),
                // D/B, 32-bit segment
                (1 << 2),
            )
        }
    };

    // Bits 8 (Segment Type) .. 15 (P)
    let type_to_p: u64 =
       // 8 .. 11 (Segment Type)
       segment_type
       // 12 (S, Descriptor Type)
       // It is set to indicate a code/data segment
       | (1 << 4)
       // 13 .. 14 (Descriptor Privilege Level)
       // Leave it as zeroes for ring 0

       // 15 (P, Segment-Present)
       // The segment is present (duh)
       | (1 << 7);

    // Bits 20 (AVL) .. 23 (G)
    let avl_to_g: u64 =
       // 20 (AVL)
       // Available for use by system software, undesirable in our case

       // 21 (L)
       // Code segment is executed in 64-bit mode
       // For DS, L bit must not be set
       l
       // 22 (D/B)
       // Indicates 32-bit, must only be set for DS
       // For CS, if the L-bit is set, then the D-bit must be cleared
       | db
       // 23 (G, Granularity)
       // Scales the limit to 4-KByte units, so we can set the limit to 4GB
       // while just occupying 20 bits overall
       // (0xFFFFF * (1024 * 4)) == ((1 << 20) << 12) == (1 << 32) == 4GB
       | (1 << 3);

    let limit: u64 = 0xFFFFF;

    // The base address will always be 0 in our case, so we don't need to
    // encode it

    let word: u64 =
        // 0 .. 8

        // 8 .. 15
        (type_to_p << 8)
        // 16 .. 19 (Top 4 bits of limit)
        | ((limit & 0xF) << 16)
        // 20 .. 23
        | (avl_to_g << 20)

        // 24 .. 31
        // 32 .. 46
    ;

    // 47 .. 64 (Bottom 16 bits of limit)
    (word << 32) | (limit >> 4)
}

fn load(mapping: *mut u8) -> u64 {
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

    // Set this field to the offset (from the beginning of the real-mode code)
    // of the end of the setup stack/heap, minus 0x0200.
    boot_params.hdr.heap_end_ptr = 0; // 0xe000 - 0x200;

    // The cmdline can be located anywhere, but we fit it in the 512 bytes
    // before the end of the heap ptr
    boot_params.hdr.cmd_line_ptr = 0x20000; // base_ptr + boot_params.hdr.heap_end_ptr as u32;

    // The 32-bit (non-real-mode) kernel starts at offset (setup_sects+1)*512
    // in the kernel file (again, if setup_sects == 0 the real value is 4.)
    let kernel_offset = (match boot_params.hdr.setup_sects as u32 {
        0 => 4,
        sects => sects,
    } + 1) as usize
        * 512;

    // Then, the setup header at offset 0x01f1 of kernel image on should be
    // loaded into struct boot_params and examined. The end of setup header
    // can be calculated as follows:
    //     0x0202 + byte value at offset 0x0201
    // 0x0201 refers to the 16 bit `jump` field of the `setup_header` struct
    // Contains an x86 jump instruction, 0xEB followed by a signed offset relative to byte 0x202
    // So we just read a byte out of it, i.e. the offset from the header (0x0202)
    let offset = boot_params.hdr.jump >> 8;

    // The offset will always be 106 unless a new field is added after
    // `kernel_info_offset`
    assert_eq!(offset, 106);

    // Dummy E820 entry, re-uses the existing mapping, will be reworked
    boot_params.e820_entries = 1;
    boot_params.e820_table[0] = boot_e820_entry {
        addr: 0,
        size: MAPPING_SIZE as u64,
        type_: E820_RAM,
    };

    let cs = get_gdt_segment(GdtSegType::Code);
    let ds = get_gdt_segment(GdtSegType::Data);

    println!("CS {cs} {cs:#0X}, DS {ds}, {ds:#0X}");

    unsafe {
        std::ptr::copy_nonoverlapping(&cs, mapping.add(0x10) as *mut u64, 1);
        std::ptr::copy_nonoverlapping(&ds, mapping.add(0x18) as *mut u64, 1);

        std::ptr::copy_nonoverlapping(boot_params, mapping.add(0x10000) as *mut boot_params, 1);

        let cmdline = b"console=ttyS0\0";

        std::ptr::copy_nonoverlapping(cmdline as *const u8, mapping.add(0x20000), cmdline.len());

        std::ptr::copy_nonoverlapping(
            kernel[kernel_offset..].as_ptr(),
            mapping.add(0x100000),
            kernel.len() - kernel_offset,
        );
    }

    use std::{fs::File, io::Write};
    File::create("/tmp/bruh")
        .unwrap()
        .write_all(&kernel[kernel_offset + 0x200..])
        .unwrap();

    0x100000 + 0x200
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

    let ip = load(*wrapped_mapping as *mut _);

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

    let cs = kvm_segment {
        base: 0,
        limit: 0xFFFFFFFF,
        selector: 0x10,
        dpl: 0,
        db: 0,
        g: 1,
        s: 1,
        l: 1,
        type_: 0xa,
        present: 1,
        ..Default::default()
    };

    let ds = kvm_segment {
        base: 0,
        limit: 0xFFFFFFFF,
        selector: 0x18,
        dpl: 0,
        db: 1,
        g: 1,
        s: 1,
        l: 0,
        type_: 0x02,
        present: 1,
        ..Default::default()
    };

    sregs.cs = cs;

    sregs.ds = ds;
    sregs.es = ds;
    sregs.fs = ds;
    sregs.gs = ds;
    sregs.ss = ds;

    sregs.gdt = kvm_dtable {
        base: 0,
        limit: 4096,
        ..Default::default()
    };
    sregs.idt = kvm_dtable::default();

    kvm.set_vcpu_sregs(&sregs)?;

    kvm.set_tss_addr(0xFFFFD000)?;

    let mut regs = kvm_regs::default();

    // Specified by x86 (reserved bit)
    // Interrupts are disabled
    regs.rflags = 1 << 1;

    // Set the instruction pointer to the start of the copied code
    regs.rip = ip;

    // unsafe { std::ptr::copy_nonoverlapping(CODE.as_ptr(), wrapped_mapping.add(ip as usize) as _, CODE.len()) }

    // boot_params
    regs.rsi = 0x10000;

    kvm.set_vcpu_regs(&regs)?;

    println!("{:#?}", kvm.get_vcpu_regs()?);

    kvm.enable_debug()?;

    println!("{:#?}", kvm.get_vcpu_events()?);

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
                        "IO for port {}: {:#02x}",
                        // TODO abstract out epic bindgen union moment
                        (*kvm_run).__bindgen_anon_1.io.port,
                        // TODO abstract out epic struct as bytes moment
                        *((kvm_run as u64 + (*kvm_run).__bindgen_anon_1.io.data_offset)
                            as *const u8)
                    );
                }
                KVM_EXIT_DEBUG => {
                    println!("{:#?}", (*kvm_run).__bindgen_anon_1.debug);
                    println!("{:#?}", kvm.get_vcpu_regs()?);
                }
                reason => {
                    eprintln!("Unhandled exit reason: {reason}");
                    break;
                }
            }
        }
    }

    println!("{:#?}", kvm.get_vcpu_events()?);

    Ok(())
}
