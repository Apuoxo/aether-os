# QEMU Ethernet validation

Status: VERIFIED — QEMU transport validation only

QEMU test target: Aether OS on stock QEMU
Physical hardware target: Fujitsu LIFEBOOK AH532 / Sandy Bridge
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

This proves that the Aether Ethernet test path boots in QEMU, initializes a Realtek RTL8139 transport, transmits and receives Ethernet frames, resolves the QEMU gateway by ARP, sends ICMP Echo, and receives the ICMP Echo Reply.

It does NOT prove Ethernet operation on the physical AH532.

Important scope:
- RTL8139 and RTL8168EVL are different controllers with different register and DMA-descriptor architectures. Only the protocol layer (ARP/IP/ICMP) is intended to be reusable.
- Stock QEMU does not expose the AH532 RTL8168EVL device. This test therefore deliberately uses RTL8139 only as a transport/stack validation path.
- The physical AH532 target remains the real Realtek 10EC:8168 device identified by the hardware probe, with XID=0x2C8. It requires independent validation on the laptop.
- Physical validation must proceed read-only first: PCI identity -> MMIO/BAR -> MAC readback -> PHY/link status -> only then DMA TX/RX -> ARP -> IP/ICMP -> ping.
- Hardware polling/wait loops must have explicit bounded iteration limits; no unbounded reset/DMA/PHY waits are acceptable.

Next hardware-validation stages:
1. On AH532, confirm BDF 2:0.0 / 10EC:8168 / XID=0x2C8.
2. Read the MAC from the real NIC and compare it with the physical adapter label.
3. Confirm PHY/link state without starting DMA.
4. Implement and verify RTL8168EVL DMA TX/RX independently of the RTL8139 path.
5. Reuse the already validated ARP/IP/ICMP logic only after Ethernet frame TX/RX is proven on the real NIC.
6. Prove ARP and then ICMP ping on the AH532.
