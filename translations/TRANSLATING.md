# Translating Caruspex

Caruspex speaks German and English in the code itself. Every other language
lives in `translations/<code>/` as JSON files and is built into the program by
`tools/gen_i18n.py`.

## Files

`translations/source/` holds what needs translating, made by
`py -I tools/i18n_extract.py`:

- `core_NN.json`: texts of the diagnosis core (fault codes with causes and
  checks, value names, protocol messages). Each entry has `id`, `de` and `en`.
  Translate from the English text; the German one helps where English is
  ambiguous.
- `ui_NN.json`: texts of the interface. Each entry has `id`, `context`, `de`
  (the source) and `en`. Entries with `"plural": true` need a list of forms.

A translation of `source/core_03.json` goes to `<code>/core_03.json`, of
`source/ui_01.json` to `<code>/ui_01.json`, as one JSON object from id to text:

```json
{"c0": "...", "c1": "..."}
{"u3": ["form for one", "form for many"]}
```

## Rules

- Keep every placeholder exactly: `%1` to `%9` and `%n`. Their order may change.
- Keep line breaks (`\n`) and leading or trailing spaces as in the English text.
- Keep technical names as they are: OBD, OBD-II, EOBD, ELM327, STN, OBDLink,
  KKL, K-Line, CAN, UDS, KWP1281, KWP2000, DoIP, ENET, VIN, PID, ECU addresses,
  fault codes like P0171, protocol names, units (V, A, kW, kWh, km/h, °C, %),
  brand and model names, Caruspex.
- Use the words a mechanic in that country would use, plain and friendly, the
  way the German and English texts talk to a car owner.
- No long dashes (the characters U+2014 and U+2013); use a comma, a colon or a
  plain hyphen instead.
- Plural forms per language: 1 for zh, ja, ko, id; 2 for es, fr, pt, it, nl, tr,
  hi; 3 for pl, cs, ru, uk (one, few, many); 6 for ar (zero, one, two, few,
  many, other).

`py -I tools/gen_i18n.py --check` reports what is missing or refused;
`py -I tools/gen_i18n.py` writes the catalog and the `.ts` files.
