# vmm

```c
$ cargo run /path/to/bzImage
early console in extract_kernel

input_data: 0x00000000018c6298

input_len: 0x0000000000093bec

output: 0x0000000001000000

output_len: 0x0000000000930650

kernel_total_size: 0x0000000000818000

needed_size: 0x0000000000a00000

trampoline_32bit: 0x0000000000000000



Decompressing Linux... Parsing ELF... done.

Booting the kernel (entry_offset: 0x0000000000000000).


Linux version 6.6.1 (testuser@shed) (gcc (GCC) 13.2.0, GNU ld (GNU Binutils) 2.41) #106 Sat Jan  6 17:52:27 IST 2024

Command line: console=ttyS0 earlyprintk=ttyS0

[Firmware Bug]: TSC doesn't count with P0 frequency!

BIOS-provided physical RAM map:

BIOS-88: [mem 0x0000000000000000-0x000000000009efff] usable

BIOS-88: [mem 0x0000000000100000-0x00000000030fffff] usable

printk: bootconsole [earlyser0] enabled

NX (Execute Disable) protection: active

APIC: Static calls initialized

tsc: Fast TSC calibration using PIT

tsc: Detected 3293.813 MHz processor

last_pfn = 0x3100 max_arch_pfn = 0x400000000

x86/PAT: PAT support disabled because CONFIG_X86_PAT is disabled in the kernel.

x86/PAT: Configuration [0-7]: WB  WT  UC- UC  WB  WT  UC- UC  

Using GB pages for direct mapping

Zone ranges:

  DMA32    [mem 0x0000000000001000-0x00000000030fffff]

  Normal   empty

Movable zone start for each node

Early memory node ranges

  node   0: [mem 0x0000000000001000-0x000000000009efff]

  node   0: [mem 0x0000000000100000-0x00000000030fffff]

Initmem setup node 0 [mem 0x0000000000001000-0x00000000030fffff]

On node 0, zone DMA32: 1 pages in unavailable ranges

On node 0, zone DMA32: 97 pages in unavailable ranges

On node 0, zone DMA32: 20224 pages in unavailable ranges

No local APIC present

APIC: disable apic facility

APIC: Switched APIC routing to: noop

[mem 0x03100000-0xffffffff] available for PCI devices

clocksource: refined-jiffies: mask: 0xffffffff max_cycles: 0xffffffff, max_idle_ns: 7645519600211568 ns

Kernel command line: console=uart,io,0x3f8 earlyprintk=serial

random: crng init done

Dentry cache hash table entries: 8192 (order: 4, 65536 bytes, linear)

Inode-cache hash table entries: 4096 (order: 3, 32768 bytes, linear)

Built 1 zonelists, mobility grouping on.  Total pages: 12092

mem auto-init: stack:all(zero), heap alloc:off, heap free:off

Memory: 38452K/49784K available (4096K kernel code, 719K rwdata, 172K rodata, 492K init, 832K bss, 11076K reserved, 0K cma-reserved)

SLUB: HWalign=64, Order=0-1, MinObjects=0, CPUs=1, Nodes=1

NR_IRQS: 4352, nr_irqs: 24, preallocated irqs: 16

APIC disabled by BIOS

APIC: Keep in PIC mode(8259)

clocksource: tsc-early: mask: 0xffffffffffffffff max_cycles: 0x2f7a784b954, max_idle_ns: 440795366813 ns

Calibrating delay loop (skipped), value calculated using timer frequency.. 6587.62 BogoMIPS (lpj=13175252)

Last level iTLB entries: 4KB 512, 2MB 512, 4MB 256

Last level dTLB entries: 4KB 2048, 2MB 2048, 4MB 1024, 1GB 0

CPU: AMD 19/50 (family: 0x19, model: 0x50, stepping: 0x0)

Spectre V1 : Mitigation: usercopy/swapgs barriers and __user pointer sanitization

Spectre V2 : Kernel not compiled with retpoline; no mitigation available!

Spectre V2 : Vulnerable

Spectre V2 : Spectre v2 / SpectreRSB mitigation: Filling RSB on context switch

Spectre V2 : Enabling Restricted Speculation for firmware calls

Spectre V2 : mitigation: Enabling conditional Indirect Branch Prediction Barrier

Speculative Store Bypass: Mitigation: Speculative Store Bypass disabled via prctl

Speculative Return Stack Overflow: IBPB-extending microcode not applied!

Speculative Return Stack Overflow: WARNING: See https://kernel.org/doc/html/latest/admin-guide/hw-vuln/srso.html for mitigation options.

Speculative Return Stack Overflow: WARNING: kernel not compiled with CPU_SRSO.

x86/fpu: Supporting XSAVE feature 0x001: 'x87 floating point registers'

x86/fpu: Supporting XSAVE feature 0x002: 'SSE registers'

x86/fpu: Supporting XSAVE feature 0x004: 'AVX registers'

x86/fpu: xstate_offset[2]:  576, xstate_sizes[2]:  256

x86/fpu: Enabled xstate features 0x7, context size is 832 bytes, using 'compacted' format.

pid_max: default: 4096 minimum: 301

Mount-cache hash table entries: 512 (order: 0, 4096 bytes, linear)

Mountpoint-cache hash table entries: 512 (order: 0, 4096 bytes, linear)

Performance Events: Fam17h+ core perfctr, 

no APIC, boot with the "lapic" boot parameter to force-enable it.

no hardware sampling interrupt available.

AMD PMU driver.

... version:                0

... bit width:              48

... generic registers:      6

... value mask:             0000ffffffffffff

... max period:             00007fffffffffff

... fixed-purpose events:   0

... event mask:             000000000000003f

signal: max sigframe size: 1360

clocksource: jiffies: mask: 0xffffffff max_cycles: 0xffffffff, max_idle_ns: 7645041785100000 ns

clocksource: Switched to clocksource tsc-early

platform rtc_cmos: registered platform RTC device (no PNP device found)

workingset: timestamp_bits=62 max_order=14 bucket_order=0

microcode: CPU0: patch_level=0x01000065

microcode: Microcode Update Driver: v2.2.

sched_clock: Marking stable (32020325, 1097167)->(32841022, 276470)

Warning: unable to open an initial console.

Freeing unused kernel image (initmem) memory: 492K

Write protecting the kernel read-only data: 6144k

Freeing unused kernel image (rodata/data gap) memory: 1876K

Run /sbin/init as init process

Run /etc/init as init process

Run /bin/init as init process

Run /bin/sh as init process

Kernel panic - not syncing: No working init found.  Try passing init= option to kernel. See Linux Documentation/admin-guide/init.rst for guidance.

Kernel Offset: disabled

---[ end Kernel panic - not syncing: No working init found.  Try passing init= option to kernel. See Linux Documentation/admin-guide/init.rst for guidance. ]---
```

## Resources

- https://lwn.net/Articles/658511

- https://github.com/firecracker-microvm/firecracker

- https://crosvm.dev/book

- https://katacontainers.io

- https://david942j.blogspot.com/2018/10/note-learning-kvm-implement-your-own.html

- https://github.com/dpw/kvm-hello-world

- https://mergeboard.com/blog/2-qemu-microvm-docker

- https://github.com/naoki9911/zig-vmm

- https://github.com/bobuhiro11/gokvm

- https://github.com/b0bleet/zvisor

- https://zserge.com/posts/kvm

- https://github.com/sysprog21/kvm-host

- https://github.com/18AX/blackhv

- https://gist.github.com/zserge/ae9098a75b2b83a1299d19b79b5fe488
