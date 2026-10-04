# QEMU Ethernet validation

Status: VERIFIED

Hardware target: Fujitsu LIFEBOOK AH532 / Sandy Bridge
Aether image: commit 9a2f0ad86a2a5fc620af99117372fe922b3fbd4c
QEMU NIC: Realtek RTL8139 transport backend
QEMU network: user mode, 10.0.2.0/24
Validation target: 10.0.2.2

GitHub Actions run: 37224938786

Serial evidence:
- [NET-QEMU] RTL8139 READY
- [NET-QEMU] ARP TX OK
- [NET-QEMU] ARP RX bytes=0x40
- [NET-QEMU] ICMP TX OK
- [NET-QEMU] ICMP RX bytes=0x62
- [NET-QEMU] PING=PASS

This proves Aether can boot in QEMU, initialize a Realtek Ethernet transport, transmit and receive Ethernet frames, resolve the QEMU gateway by ARP, send ICMP Echo, and receive the ICMP Echo Reply.

Important scope:
- The physical AH532 RTL8168EVL path remains separate and still requires validation against the real 10EC:8168/XID=0x2C8 device.
- RTL8139 is used here as a QEMU-compatible transport validation path because stock QEMU does not expose the AH532 RTL8168EVL device.
