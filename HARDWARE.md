# IoT Switch — Hardware Guide

## Shopping List

### DIN Rail Components (mount on rail)

| Item | Purpose | Example |
|------|---------|---------|
| **MCB** (6-10A, single-pole) | Overload/short protection for pump | Schneider iC60N 10A |
| **Contactor** (230V AC coil, 2-pole, 20A+) | Switches pump mains | Eaton DILM7-10 (230V) |
| **DIN rail terminal blocks** (4-6 pieces) | Neat wiring junctions | Wago 4mm² feed-through |
| **35mm DIN rail** (8-12 inch) | Mounting rail | Any brand, steel |
| **DIN rail enclosure** (IP65, ~4-6 module slots) | Weatherproof box | Spelsberg TK series |

### Low-Voltage Side (Pi + relay)

| Item | Purpose | Example |
|------|---------|---------|
| Raspberry Pi Zero 2W | Controller | Already have |
| Micro SD card 16GB+ | OS + software | Samsung EVO Plus |
| 5V 2.5A USB-C PSU | Pi power | Official Pi PSU |
| **1-channel relay module** (5V active-low) | Drives contactor coil | Songle SRD-05VDC-SL-C |
| **DIN rail mount for Pi Zero** | Mount Pi on rail | 3D-printed bracket or this |
| Female-female jumper wires (3) | Pi → Relay | Dupont wires |
| Flexible wire 1.5mm² (1m) | Relay → Contactor → Pump | H07V-K 1.5mm² |
| **Cable glands** (PG9/PG11) | Entry/exit from enclosure | 2-4 pieces |

### Optional

| Item | Purpose |
|------|---------|
| **24V DIN rail PSU** (Mean Well HDR-15-24) | Powers Pi via 24V→5V step-down — cleaner than USB wall wart |
| **Emergency stop button** (red mushroom, push-to-break) | Physical kill switch |
| **Status indicator light** (230V neon) | Shows pump running from outside box |

---

## DIN Rail Layout

```
┌─────────────────────────────────────────────────────┐
│                   DIN Rail Enclosure                 │
│  ┌──────┐  ┌──────────┐  ┌──────────────┐  ┌────┐  │
│  │ MCB  │  │ Terminal │  │  Contactor   │  │    │  │
│  │ 10A  │──│  Blocks  │──│ DILM7-10     │  │ Pi │  │
│  │      │  │ (wire →  │  │  (230V coil) │  │ +  │  │
│  │  IN ─┼──┤  pump)   │  │              │  │Rel │  │
│  │      │  └──────────┘  │  Power ───────┼──┤ay  │  │
│  │ OUT ─┼────────────────┤  contacts     │  │    │  │
│  └──────┘                │              │  └────┘  │
│                          │  Coil A1 ◄───┤          │
│                          │  Coil A2 ────┤ (neutral)│
│                          └──────────────┘          │
└─────────────────────────────────────────────────────┘
```

Components snap onto the DIN rail in order: **MCB → terminals → contactor → Pi mount**.

---

## Wiring Diagram (step by step)

### Low Voltage (5V — Pi section)

```
Pi Zero 2W              Relay Module
┌──────────┐          ┌───────────┐
│ GPIO 17  ───────────► IN        │
│ 5V (pin2)───────────► VCC       │
│ GND (pin6)──────────► GND       │
└──────────┘          └─────┬─────┘
                            │
                 COM ───────┤
                 NO  ───────┼─── to Contactor coil A1
                 NC  ──┐    │
                       │    │
                   (unused) │
```

### Mains Voltage (230V AC)

```
Mains In                  MCB                 Contactor               Pump
┌────────┐           ┌──────────┐         ┌──────────────┐        ┌────────┐
│ L ─────┼───────────┤ IN    OUT├─────────┤ Power 1 ─────┼────────┤ L      │
│        │           └──────────┘         │              │        │        │
│        │                                │ Power 3 ─────┼────────┤ N      │
│        │                                │              │        └────────┘
│        │                                │              │
│        │           Contactor Coil       │              │
│        │           ┌────────────┐       │              │
├────────┼───────────┤ A1  (from  │       │              │
│        │           │  relay NO) │       │              │
│        │           │            │       │              │
│ N ─────┼───────────┤ A2  (N)    │       │              │
└────────┘           └────────────┘       └──────────────┘
```

---

## How it works

1. **Pi GPIO 17 → LOW** → Relay activates
2. **Relay COM→NO closes** → 230V flows from L to contactor coil A1
3. **Contactor coil energizes** → Main power contacts 1-2 and 3-4 close
4. **Pump runs**

When GPIO goes HIGH, everything reverses and pump stops.

---

## Safety

- The Pi and relay (5V side) are **galvanically isolated** from mains
- **MCB** protects against overload and short circuit
- **Contactor** provides physical air-gap isolation when off
- **RCD/GFCI** on the pump circuit is strongly recommended
- All mains wiring must comply with local electrical codes
- Use **cable glands** where wires enter/exit the enclosure
- Label all terminals clearly inside the box

---

## Alternative: Solid State Relay (SSR)

Instead of a mechanical contactor, use **SSR-40 DA** (40A, 24-380V AC). The SSR is driven by the relay module's NO contacts. Silent, no moving parts, but generates heat under load (needs heatsink).

```
Pi GPIO → Relay module → SSR control input (3-32V DC)
                           └── SSR output → Pump
```
