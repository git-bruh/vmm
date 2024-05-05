# vmm

Tiny VMM that boots Linux

## Usage

A sample kernel config and init is present in the `contrib` directory. An initramfs can be created like so:

```sh
$ cc contrib/init.c -o init -static
# cpio takes the file list from stdin
$ echo init | cpio -o -H newc > initramfs
```

**NOTE:** By default, the code prints out every byte received on the serial ports, which can be suppressed by redirecting stderr to `/dev/null`

`cargo run <KERNEL_IMAGE> <INITRAMFS>`

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
