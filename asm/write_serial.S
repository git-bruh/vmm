BITS 64

mov rbx, msg
mov rdx, 0x3f8

print:
  mov rax, [rbx]

  ;; print till NUL terminator
  cmp rax, 0x0
  je end

  ;; dx, al correspond to rax, rdx
  out dx, al

  inc rbx
  jmp print

end:
  hlt

msg:
  db `Hello, KVM!\n`
