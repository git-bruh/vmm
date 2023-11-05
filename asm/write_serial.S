;; output to the 0x3f8 serial port
mov dx, 0x3f8

;; add the al and bl registers and get the ASCII representation
;; by adding 48 ('0')
add al, bl
add al, 48
out dx, al

;; output a newline, 10 ('\n')
mov al, 10
out dx, al

hlt
