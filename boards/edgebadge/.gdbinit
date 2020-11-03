
# print demangled symbols by default
set print asm-demangle on

# JLink
target extended-remote :2331
monitor flash breakpoints 1

monitor halt
load
# reset *after* loading so that we run through the bootloader
# and correctly bootstrap the loaded code
monitor reset

#break usb_serial::usb_handler
break HardFault
break DefaultHandler

# OpenOCD
#target extended-remote :3333
#monitor arm semihosting enable
#load
#step
c