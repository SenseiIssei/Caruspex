# सुरक्षा, ट्रेडमार्क और लाइसेंस

## सुरक्षा और दायित्व

Caruspex एक मुफ़्त शौकिया परियोजना है और, जैसा [MIT लाइसेंस](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) में लिखा है, बिना किसी वारंटी के दी जाती है। इसका उपयोग आप अपने जोखिम पर करते हैं।

- गाड़ी चलाते समय इसे कभी न चलाएँ। इसे खड़ी गाड़ी में सेट करें; लाइव मान कोई सहयात्री देख सकता है।
- फ़ॉल्ट मिटाना, एक्चुएटर टेस्ट और बेसिक सेटिंग गाड़ी से कुछ करवाते हैं। Caruspex इनमें से हर एक से पहले पूछता है; इन्हें केवल खड़ी और सुरक्षित गाड़ी में करें, और केवल उन्हीं वाहनों पर जिन पर काम करने की आपको अनुमति है।
- Caruspex किसी वर्कशॉप का विकल्प नहीं है। फ़ॉल्ट कोड एक लक्षण बताता है, हमेशा उसका कारण नहीं। ब्रेक, एयरबैग, स्टीयरिंग और सुरक्षा से जुड़ी हर चीज़ विशेषज्ञों के हाथ में होनी चाहिए।
- मिटाना मरम्मत नहीं है। मिटाए गए फ़ॉल्ट और रीसेट किए गए रेडीनेस मॉनिटर के कारण उत्सर्जन या फ़िटनेस जाँच फ़ेल हो सकती है।
- Caruspex माइलेज, इमोबिलाइज़र, चाबियाँ, उत्सर्जन नियंत्रण या टैकोग्राफ़ नहीं बदलता, और परियोजना ऐसे योगदान स्वीकार नहीं करती जो यह करें। ओडोमीटर, उत्सर्जन नियंत्रण या टैकोग्राफ़ से छेड़छाड़ कई देशों में ग़ैरक़ानूनी है।

## ट्रेडमार्क

वाहन निर्माताओं, मॉडलों, अडैप्टरों, प्रोग्रामों और मानकों के नाम (जैसे Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS या ISTA) उनके संबंधित स्वामियों के ट्रेडमार्क हैं। ये केवल यह बताने के लिए दिए गए हैं कि Caruspex किन कारों, अडैप्टरों और इंटरफ़ेस के साथ काम करता है। Caruspex एक स्वतंत्र परियोजना है और किसी भी वाहन निर्माता, अडैप्टर निर्माता या सॉफ़्टवेयर विक्रेता से जुड़ी, उसके द्वारा समर्थित या प्रायोजित नहीं है। ऐप और यह रिपॉज़िटरी इन कंपनियों के लोगो का उपयोग नहीं करते।

## लाइसेंस और स्रोत

कोड MIT लाइसेंस के तहत है, [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) देखें। प्रोग्रामों में शामिल लाइब्रेरी और उनके लाइसेंस [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md) में दिए गए हैं; ऐप इन्हें सेटिंग्स में दिखाता है। प्रोग्रामों में शामिल डेटा:

- लगभग 550 वाहनों के निर्माता मान, [OBDb](https://github.com/OBDb) से बदले गए, OBDb की तरह CC BY-SA 4.0 के तहत: कोड से अलग [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) में लाइसेंस और स्रोतों की सूची ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)) के साथ।
- CAN सिग्नल की परिभाषाएँ [opendbc](https://github.com/commaai/opendbc) (comma.ai) और [model3dbc](https://github.com/joshwardell/model3dbc) से, MIT लाइसेंस ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla))।
- इंजन कोड विकिपीडिया लेखों से, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), हर पंक्ति में लेख और संस्करण के साथ)।
- VIN के निर्माता कोड का एक हिस्सा [विकिपुस्तक की WMI सूची](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) के अनुसार, CC BY-SA 4.0।
- वाहनों के माप [विकिडेटा](https://www.wikidata.org) से, CC0।
- SIL Open Font License 1.1 के तहत फ़ॉन्ट: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt))।
