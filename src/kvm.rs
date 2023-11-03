use crate::util::WrappedAutoFree;
use core::num::NonZeroUsize;
use kvm_bindings::{kvm_regs, kvm_run as kvm_run_t, kvm_sregs, kvm_userspace_memory_region, KVMIO};
use nix::{
    fcntl,
    fcntl::OFlag,
    sys::{mman, mman::MapFlags, mman::ProtFlags, stat::Mode},
};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

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
ioctl_write_ptr!(kvm_set_regs, KVMIO, 0x82, kvm_regs);
ioctl_read!(kvm_get_sregs, KVMIO, 0x83, kvm_sregs);
ioctl_write_ptr!(kvm_set_sregs, KVMIO, 0x84, kvm_sregs);

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

    pub fn set_vcpu_regs(&self, regs: *const kvm_regs) -> Result<(), std::io::Error> {
        unsafe { kvm_set_regs(self.vcpu.as_raw_fd(), regs)? };

        Ok(())
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
