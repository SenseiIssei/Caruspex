# Generic fault codes: tiers and codes left out

How sure each generic code in `tools/dtc/extended.csv` is, and which codes of the obdex data set
(CC0, https://github.com/foerbsnavi/OBDex, data of 2026-11-07) are not in the table and why. Codes
that are not in the table still show the group meaning of their range in the app.

## Tiers

- **confirmed**: obdex and a manufacturer document name the code with the same meaning: the GM OBD
  descriptions (https://gsi.ext.gm.com/gmspo/mode6/pdf/, 117 documents) or a bulletin or investigation
  document in the public NHTSA archive (https://static.nhtsa.gov/odi/, 2171 documents). Checked by script
  on 2026-11-20 in two passes; the second pass reads abbreviations and synonyms (Ckt, Sw, Sol, ECT, IAT,
  LF/RF and so on). The code must appear in the document with the words of the part and every side or
  position word (left, rear, intake, upstream) next to it.
- **single**: only obdex defines the code, and obdex does not itself say that the title is missing from
  the public J2012 lists. The app and the reports show "Meaning from one source" next to the text.
- The 1379 written-out codes in `tools/dtc/generic.py` predate this work and carry no tier.

| Block | Written out | Confirmed | One source |
|---|---:|---:|---:|
| B0000-B0FFF | 6 | 18 | 259 |
| C0000-C0FFF | 5 | 139 | 445 |
| P0000-P09FF | 887 | 528 | 1008 |
| P0A00-P0FFF | 10 | 509 | 753 |
| P2000-P29FF | 316 | 544 | 1402 |
| P2A00-P2FFF | 3 | 128 | 1079 |
| P3400-P34FF | 66 | 2 | 86 |
| U0000-U0FFF | 86 | 141 | 823 |
| U3000-U3FFF | 0 | 18 | 154 |
| **Total** | **1379** | **2027** | **6009** |

To promote a code from single to confirmed: find a second source that names it with the same meaning,
change its tier in `tools/dtc/extended.csv` and run `py -I tools/gen_dtc.py`.

Ford's OBD summaries (fordservicecontent.com) refuse automated downloads (403) and were not used.

## Existing entries with disagreeing sources

- **C0040, C0045**: removed. Caruspex listed them as wheel speed sensors right front and left rear (GM's
  use). obdex and NHTSA bulletins from 2018 and 2023 give brake pedal switch A and brake pressure sensor B.
- **C0035**: kept for now, owner to decide. Caruspex and GM: left front wheel speed sensor; obdex: right
  front wheel speed sensor supply. The demo car and several tests use C0035 as a wheel speed fault.

## Left out

**obdex itself doubts the title (102)**: its description says the number is absent from the public
J2012 lists or unverified (often a maker uses the number for something else):

B0007, B0014, B0015, B0023, B0025, B0034, B00C7, B0182, B0186, B0205, B0220, B0223, B0231, B0252, B0258, B0270, B0283, B0288, B0292, B0325, B0342, B0420, B0440, B0445, B0450, B0455, B0500, B0501, B0503, B0520, B0530, B0531, B0533, B0540, B0560, B0580, B0610, B0700, B0920, B0960, C0111, C0128, C0131, C0136, C0148, C0151, C0156, C0158, C0173, C0202, C0238, C0240, C0244, C0265, C0268, C0279, C0288, C0301, C0307, C0420, C0440, C0445, C0460, C0496, C054B, C0670, C0680, C0712, C0735, C0745, C0852, C0860, C0900, C0916, C0952, P08E2, P08E3, P0B8F, P0B94, P0BBC, P0E1D, P0E91, P0E9B, P0EAD, P20A7, P241E, P24FD, P25E8, P278E, P28EF, P2C06, P2CC8, P2D08, P2DCD, P34C0, U0097, U0136, U019C, U0474, U056B, U3014, U355F

**No meaning (1)**: P0FFF, obdex marks it as undefined.

**Left out for wording (13)**: the meaning is clear, but the composed text would split it
(NOx sensor slow or delayed response low to high and high to low, clutch pressure "performance/too low");
each needs a phrase of its own: P22F9, P22FA, P22FC, P22FD, P23BF, P23C0, P23C2, P24F4, P24F5, P2857, P2858, P2859, P285A.

Some groups in obdex look like guesses rather than SAE codes (for example parts of B0400 to B09FF,
C0100 to C04FF and C0800 to C09FF); where obdex says so itself they are left out above, the rest are in the
single tier. The few confirmed codes there (C0186, C0196, C0242, C0245, C0277, C0287, C0306, C0387, C0455,
C0800, C0870, B0125, B0158, B0159) follow GM's use and may mean something else on other makes.
