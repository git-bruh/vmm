BITS 64

in al,0xee

mov rbx, msg
mov rdx, 0x3f8

print:
  mov rax, [rbx]
  push rbx

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
  db `Hello, KVM!`
