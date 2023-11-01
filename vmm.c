#include <assert.h>
#include <fcntl.h>
#include <linux/kvm.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>

static const uint8_t code[] = {
    0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
    0x00, 0xd8,       /* add %bl, %al */
    0x04, '0',        /* add $'0', %al */
    0xee,             /* out %al, (%dx) */
    0xb0, '\n',       /* mov $'\n', %al */
    0xee,             /* out %al, (%dx) */
    0xf4,             /* hlt */
};

int main(void) {
  int kvm = open("/dev/kvm", O_RDWR);
  assert(kvm != -1);

  int vm = ioctl(kvm, KVM_CREATE_VM, 0UL);
  assert(vm != -1);

  void *mem = mmap(NULL, 0x1000, PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
  assert(mem);

  memcpy(mem, code, sizeof(code));

  struct kvm_userspace_memory_region region = {.slot = 0,
                                               .guest_phys_addr = 0x1000,
                                               .memory_size = 0x1000,
                                               .userspace_addr =
                                                   (uintptr_t)mem};

  int ret = ioctl(vm, KVM_SET_USER_MEMORY_REGION, &region);
  assert(ret != -1);

  int vcpu = ioctl(vm, KVM_CREATE_VCPU, 0UL);
  assert(vcpu != -1);

  int mmap_size = ioctl(kvm, KVM_GET_VCPU_MMAP_SIZE, NULL);
  assert(mmap_size != -1);

  struct kvm_run *run =
      mmap(NULL, mmap_size, PROT_READ | PROT_WRITE, MAP_SHARED, vcpu, 0);
  assert(run);

  struct kvm_sregs sregs;

  ret = ioctl(vcpu, KVM_GET_SREGS, &sregs);
  assert(ret != -1);

  sregs.cs.base = 0;
  sregs.cs.selector = 0;

  ret = ioctl(vcpu, KVM_SET_SREGS, &sregs);
  assert(ret != -1);

  struct kvm_regs regs = {
      .rip = 0x1000,
      .rax = 4,
      .rbx = 2,
      .rflags = 0x2,
  };

  ret = ioctl(vcpu, KVM_SET_REGS, &regs);
  assert(ret != -1);

  for (;;) {
    ret = ioctl(vcpu, KVM_RUN, NULL);
    assert(ret != -1);

    switch (run->exit_reason) {
    case KVM_EXIT_HLT:
      munmap(mem, 0x1000);
      close(kvm);

      return 0;
    case KVM_EXIT_IO:
      if (run->io.direction == KVM_EXIT_IO_OUT && run->io.size == 1 &&
          run->io.port == 0x3f8 && run->io.count == 1) {
        putchar(*(((char *)run) + run->io.data_offset));
      }

      break;
    default:
      printf("Failed with ret %d\n", ret);
      assert(0);
    }
  }
}
