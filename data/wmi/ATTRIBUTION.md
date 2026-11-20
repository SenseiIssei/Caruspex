# WMI codes after Wikibooks

The file `wmi.csv` in this folder lists world manufacturer identifiers (the
first three characters of a VIN) after the page
[Vehicle Identification Numbers (VIN codes)/World Manufacturer Identifier (WMI)](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_(VIN_codes)/World_Manufacturer_Identifier_(WMI))
on Wikibooks, revision 4682957 of 2026-11-19, by the Wikibooks contributors
(the authors are listed in the
[page history](https://en.wikibooks.org/w/index.php?title=Vehicle_Identification_Numbers_(VIN_codes)/World_Manufacturer_Identifier_(WMI)&action=history)).
The page and this adaptation are licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/)
licence, see `LICENSE` next to this file. This folder is not covered by the
MIT licence of the Caruspex code.

Changes made: only the code and the maker were taken. The maker names as
Caruspex shows them, the brand lists and the rules for the model year and
the model code are Caruspex's own. `tools/gen_wmi.py` turns the file into
`crates/caruspex-vehicles/src/generated/wmi_cc_by_sa.rs`.

Only codes that neither NHTSA vPIC (public domain) nor Wikidata (CC0) have
are in this file. The table in `crates/caruspex-vehicles/src/vin.rs` holds
the codes those sources confirm and is under the MIT licence of the code.
