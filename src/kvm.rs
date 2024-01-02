use crate::util::WrappedAutoFree;
use core::num::NonZeroUsize;
use kvm_bindings::{
    kvm_enable_cap, kvm_guest_debug, kvm_pit_config, kvm_regs, kvm_run as kvm_run_t, kvm_sregs,
    kvm_userspace_memory_region, kvm_vcpu_events, KVMIO, KVM_GUESTDBG_ENABLE,
    KVM_GUESTDBG_SINGLESTEP,
};
use nix::{
    errno::Errno,
    fcntl,
    fcntl::OFlag,
    libc,
    sys::{mman, mman::MapFlags, mman::ProtFlags, stat::Mode},
};
use std::{
    ffi::c_int,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

ioctl_write_int_bad!(kvm_create_vm, request_code_none!(KVMIO, 0x01));
ioctl_write_int_bad!(kvm_get_vcpu_mmap_size, request_code_none!(KVMIO, 0x04));
ioctl_write_int_bad!(kvm_run, request_code_none!(KVMIO, 0x80));
ioctl_write_int_bad!(kvm_create_vcpu, request_code_none!(KVMIO, 0x41));
ioctl_write_ptr!(
    kvm_set_user_memory_region,
    KVMIO,
    0x46,
    kvm_userspace_memory_region
);
ioctl_read!(kvm_get_regs, KVMIO, 0x81, kvm_regs);
ioctl_write_ptr!(kvm_set_regs, KVMIO, 0x82, kvm_regs);
ioctl_read!(kvm_get_sregs, KVMIO, 0x83, kvm_sregs);
ioctl_write_ptr!(kvm_set_sregs, KVMIO, 0x84, kvm_sregs);
ioctl_none!(kvm_create_irqchip, KVMIO, 0x60);
ioctl_write_ptr!(kvm_create_pit2, KVMIO, 0x77, kvm_pit_config);
ioctl_write_ptr!(kvm_set_guest_debug, KVMIO, 0x9b, kvm_guest_debug);
ioctl_write_ptr!(kvm_enable_capability, KVMIO, 0xa3, kvm_enable_cap);
ioctl_read!(kvm_get_vcpu_events, KVMIO, 0x9f, kvm_vcpu_events);

/*
   Blocked on https://github.com/nix-rust/nix/pull/2233
   ioctl_write_int_bad!(kvm_set_tss_addr, request_code_none!(KVMIO, 0x47));
*/

ioctl_write_ptr!(kvm_set_identity_map_addr, KVMIO, 0x48, u64);

unsafe fn kvm_set_tss_addr(fd: c_int, data: u64) -> nix::Result<c_int> {
    Errno::result(libc::ioctl(fd, request_code_none!(KVMIO, 0x47), data))
}

pub struct Kvm {
    _kvm: OwnedFd,
    vm: OwnedFd,
    vcpu: OwnedFd,
    kvm_run: WrappedAutoFree<*mut kvm_run_t, Box<dyn FnOnce(*mut kvm_run_t)>>,
}

impl Kvm {
    pub fn new() -> Result<Self, std::io::Error> {
        let kvm =
            unsafe { OwnedFd::from_raw_fd(fcntl::open("/dev/kvm", OFlag::O_RDWR, Mode::empty())?) };
        let vm = unsafe { OwnedFd::from_raw_fd(kvm_create_vm(kvm.as_raw_fd(), 0)?) };

        unsafe {
            kvm_create_irqchip(vm.as_raw_fd())?;
            kvm_create_pit2(vm.as_raw_fd(), &kvm_pit_config::default())?;

            let idmap_addr = 0xFFFFC000;
            kvm_set_identity_map_addr(vm.as_raw_fd(), &idmap_addr)?;
        };

        let vcpu = unsafe { OwnedFd::from_raw_fd(kvm_create_vcpu(vm.as_raw_fd(), 0)?) };

        let mmap_size = NonZeroUsize::new(unsafe {
            kvm_get_vcpu_mmap_size(kvm.as_raw_fd(), 0)?
                .try_into()
                .expect("KVM provided mmap_size doesn't fit usize!")
        })
        .expect("KVM provided zero usize!");

        let kvm_run = WrappedAutoFree::new(
            unsafe {
                mman::mmap(
                    None,
                    mmap_size,
                    ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                    MapFlags::MAP_SHARED,
                    Some(&vcpu),
                    0,
                )? as *mut kvm_run_t
            },
            Box::new(move |map: *mut kvm_run_t| unsafe {
                mman::munmap(map as _, mmap_size.get()).expect("failed to unmap kvm_run!");
            }) as _,
        );

        Ok(Self {
            _kvm: kvm,
            vm,
            vcpu,
            kvm_run,
        })
    }

    pub fn set_user_memory_region(
        &self,
        guest_addr: u64,
        memory_size: u64,
        map_addr: u64,
    ) -> Result<(), std::io::Error> {
        unsafe {
            kvm_set_user_memory_region(
                self.vm.as_raw_fd(),
                &kvm_userspace_memory_region {
                    slot: 0,
                    flags: 0,
                    guest_phys_addr: guest_addr,
                    memory_size: memory_size,
                    userspace_addr: map_addr,
                },
            )?;
        }

        Ok(())
    }

    pub fn get_vcpu_sregs(&self) -> Result<kvm_sregs, std::io::Error> {
        let mut sregs = kvm_sregs::default();
        unsafe { kvm_get_sregs(self.vcpu.as_raw_fd(), &mut sregs)? };

        Ok(sregs)
    }

    pub fn set_vcpu_sregs(&self, sregs: *const kvm_sregs) -> Result<(), std::io::Error> {
        unsafe { kvm_set_sregs(self.vcpu.as_raw_fd(), sregs)? };

        Ok(())
    }

    pub fn get_vcpu_regs(&self) -> Result<kvm_regs, std::io::Error> {
        let mut regs = kvm_regs::default();
        unsafe { kvm_get_regs(self.vcpu.as_raw_fd(), &mut regs)? };

        Ok(regs)
    }

    pub fn set_vcpu_regs(&self, regs: *const kvm_regs) -> Result<(), std::io::Error> {
        unsafe { kvm_set_regs(self.vcpu.as_raw_fd(), regs)? };

        Ok(())
    }

    pub fn set_tss_addr(&self, addr: u64) -> Result<(), std::io::Error> {
        unsafe { kvm_set_tss_addr(self.vm.as_raw_fd(), addr)? };

        Ok(())
    }

    pub fn enable_debug(&self) -> Result<(), std::io::Error> {
        let mut dbg = kvm_guest_debug {
            control: KVM_GUESTDBG_ENABLE | KVM_GUESTDBG_SINGLESTEP,
            ..Default::default()
        };

        dbg.arch.debugreg[7] = 0x00000400;

        unsafe {
            kvm_set_guest_debug(self.vcpu.as_raw_fd(), &dbg)?;
        }

        unsafe {
            kvm_enable_capability(
                self.vm.as_raw_fd(),
                &kvm_enable_cap {
                    // KVM_CAP_X86_TRIPLE_FAULT_EVENT
                    cap: 218,
                    ..Default::default()
                },
            )?;
        }

        Ok(())
    }

    pub fn get_vcpu_events(&self) -> Result<kvm_vcpu_events, std::io::Error> {
        let mut events = kvm_vcpu_events::default();

        unsafe {
            kvm_get_vcpu_events(self.vcpu.as_raw_fd(), &mut events)?;
        }

        Ok(events)
    }

    pub fn run(&self) -> Result<*const kvm_run_t, std::io::Error> {
        unsafe {
            kvm_run(self.vcpu.as_raw_fd(), 0)?;
        }

        // The `kvm_run` struct is filled with new data as it was associated
        // with the `vcpu` FD in the mmap() call
        Ok(*self.kvm_run as *const kvm_run_t)
    }
}
