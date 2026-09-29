# Network

Сетевая документация Aether OS для целевого ноутбука Fujitsu LIFEBOOK AH532.

## Current status

- Wi-Fi: Intel Centrino Wireless-N 2230 (8086:0887, subsystem 8086:4062) — firmware/ALIVE/command-TX bring-up, scan/networking incomplete.
- Ethernet: Realtek 10ec:8168 family — PCI/device discovery exists; complete native network driver is not finished.
- Network API: общий native network layer ещё стабилизируется.
- DHCP/IP/ARP: после появления рабочего сетевого интерфейса.
- BIOS/ACPI radio state: отдельное исследование.

Подробный Wi-Fi статус находится в WIFI_AH532.md и docs/WIFI_BRINGUP_LOG.md. Для текущей общей картины использовать docs/STATUS.md.
