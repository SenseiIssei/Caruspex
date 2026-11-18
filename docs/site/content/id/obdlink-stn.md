# OBDLink dan chip STN

Adaptor OBDLink (SX, EX, MX+, CX, LX) memakai chip STN dari OBD Solutions. Chip ini memahami semua yang bisa dilakukan ELM327 dan lebih; Caruspex mengenalinya sendiri dan memakai fitur tambahannya.

## Yang dipakai Caruspex

- **Permintaan sekaligus**: Caruspex menanyakan sampai 24 nilai langsung sekaligus, bukan 6. Lewat Bluetooth ini membuat satu putaran nilai hampir dua kali lebih cepat.
- **Mendengarkan**: nilai siaran (misalnya di Tesla) direkam dengan perintah khusus STN, yang berhenti sendiri setelah sejumlah pesan tertentu.
- **Ford MS-CAN dan GM Single-Wire-CAN**: unit kontrol bodi Ford dan GM tidak berada di CAN diagnosis biasa. Dengan adaptor yang punya transceiver tambahan, halaman **Semua unit kontrol** menawarkan "Ford: unit bodi di MS-CAN" atau "GM: unit bodi di Single-Wire-CAN".

| Chip | Adaptor | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 sampai STN2232 | OBDLink EX | ya | tidak |
| STN1150 sampai 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ dan lainnya | ya | ya |
| lainnya | OBDLink SX, LX, CX | tidak | tidak |

## Menyambungkan

Adaptor OBDLink disambungkan seperti ELM327 lainnya: lewat USB (EX dan SX), Bluetooth (MX+, LX, CX), atau seperti dijelaskan di [ELM327 lewat USB](elm327-usb.md) dan [ELM327 lewat Bluetooth](elm327-bluetooth.md).

## Sudah diuji

Satu OBDLink dengan chip STN2232 lulus tes adaptor: chip, firmware, dan perintahnya dikenali dengan benar. Adaptor ini belum dipakai di mobil, jadi permintaan sekaligus, MS-CAN, dan SW-CAN belum terkonfirmasi di perangkat keras sungguhan.
