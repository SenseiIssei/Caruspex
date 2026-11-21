# Sources and licences of the hand-written PID packs

Every value in the packs of this folder names its source in the `# Source:`
line above it. Only sources whose licence allows shipping the data in an MIT
program are used (see CONTRIBUTING.md); `tools/gen_pids.py` refuses source
lines that name projects under the GPL, under a non-commercial licence or
without a licence.

## Packs under CC BY-SA 4.0

These packs adapt signal definitions of [OBDb](https://github.com/OBDb), the
open vehicle signal database, by the OBDb contributors, and are therefore
licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/)
licence (`LICENSE-CC-BY-SA-4.0` next to this file), not under the MIT licence
of the Caruspex code. Each carries the line `#@ licence: CC BY-SA 4.0`.

| Pack | OBDb repository and commit |
|---|---|
| `hyundai-ioniq-28.csv` | github.com/OBDb/Hyundai-IONIQ-Electric at e4a37169e7d049dcf95516b76465c94c1cf01743 |
| `hyundai-kona-niro-ev.csv` | github.com/OBDb/Hyundai-Kona-Electric at c548226682d9e7f56f6bc81b37ba5971385b4f8d, github.com/OBDb/Kia-Niro-EV at 0991366cb6a434b030ddfc14b69d165ece9942f4 |
| `nissan-leaf-ze0-aze0.csv` | github.com/OBDb/Nissan-Leaf at 860cb5ab22d4587c28a1679c966351c15a8c7586 |
| `renault-zoe-ph1.csv` | github.com/OBDb/Renault-ZOE at 790c689ac3d36f69485bee9821bc5cdd7456f955 |

Changes made: the signal definitions (bit position, length, scale, offset,
sign) were rewritten as Torque equations over the answer bytes, with our own
names in German and English and our own value ranges; values from the other
sources below were added.

## MIT sources

- Open Vehicle Monitoring System 3,
  https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3 at commit
  7470869a2db574ad1fab962a3cae4f22e6cf1385: the decoding code of
  vehicle_renaultzoe, vehicle_hyundai_ioniqvfl, vehicle_kianiroev and
  vehicle_nissanleaf; vehicle_byd_atto3 only to confirm the BYD Atto 3 values
  in data/obdb and the sign of its battery current (see FALLBACK and
  METRIC_BY_ID in tools/gen_obdb.py). Licence and copyright lines:
  `LICENSE-OVMS`.
- evDash, https://github.com/nickn17/evDash at commit
  0f4ca249ff10b539558fa158b4b52b1f8e5f3e1f: src/CarRenaultZoe.cpp. Licence:
  `LICENSE-evDash`.
- model3dbc, https://github.com/joshwardell/model3dbc (Tesla pack). Licence:
  `data/tesla/LICENSE-model3dbc`.

## Removed in 2026-11

Two values of the Renault Zoe pack had no source other than CanZE (GPL-3.0)
and were removed: the state of health of the vehicle controller (EVC, 22 3206)
and the average battery temperature of the battery controller (21 04, byte
BV). The Hyundai packs no longer rest on EVNotify (CC BY-NC 4.0) or the
JejuSoul PID lists (no licence); every value there is now backed by OBDb and
most by OVMS as well.
