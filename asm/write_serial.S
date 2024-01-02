BITS 64

cld
cli

xor eax, eax
mov ds, eax
mov es, eax
mov ss, eax
mov fs, eax
mov gs, eax

push 0
popf

lea rsi, [rel 0x8de12]
hlt
