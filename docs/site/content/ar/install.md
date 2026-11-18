# التثبيت

كل إصدار موجود على [صفحة الإصدارات في GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). الحزم غير موقّعة بعد، لذلك يسألك Windows وmacOS مرة واحدة قبل التشغيل الأول.

## Windows 10 و11

| الملف | ما هو |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | برنامج التثبيت. لا يحتاج إلى صلاحيات المسؤول، ويمكنه إضافة أداة سطر الأوامر `caruspex_cli` إلى مسار البحث، ولا يثبّت بيئة WebView2 إلا إذا كانت مفقودة. |
| `Caruspex-<version>-windows-x64.zip` | نسخة محمولة: فك الضغط وشغّل `caruspex.exe`. |

عند التشغيل الأول قد يقول Windows SmartScreen إن التطبيق غير معروف: انقر "مزيد من المعلومات" ثم "التشغيل على أي حال".

تحتاج محوّلات USB إلى تعريف لشريحة USB الخاصة بها. عادةً يجده Windows 10 و11 بنفسيهما؛ وإن لم يحدث، فراجع [ELM327 عبر USB](elm327-usb.md).

## macOS 13 وما بعده (Apple Silicon)

نزّل `Caruspex-<version>-macos.dmg` وافتحه واسحب Caruspex إلى مجلد التطبيقات. لأن التطبيق لم يُوثَّق لدى Apple بعد، افتحه في المرة الأولى بالنقر بزر الفأرة الأيمن ثم "فتح".

## Linux (x86_64)

| الملف | ما هو |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | يعمل على كل التوزيعات تقريبًا: اجعله قابلًا للتنفيذ بالأمر `chmod +x` ثم شغّله. وهو يحدّث نفسه أيضًا. |
| `Caruspex-<version>-linux-amd64.deb` | لـ Debian وUbuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

لاستخدام المحوّلات التسلسلية من دون `sudo`، أضف نفسك مرة واحدة إلى المجموعة `dialout`، ثم سجّل الخروج وادخل من جديد:

```
sudo usermod -aG dialout $USER
```

## أداة سطر الأوامر فقط

للسكربتات والخوادم توجد الأداة `caruspex_cli` من دون واجهة، لـ Windows وLinux وmacOS. الأمر `caruspex_cli help` يعرض قائمة الأوامر.

## التحديثات

عند التشغيل يسأل Caruspex موقع GitHub هل يوجد إصدار أحدث، ولا شيء غير ذلك. لا يُثبَّت الإصدار الجديد إلا إذا طابق توقيعه المفتاح المدمج في التطبيق. يمكن إيقاف الفحص والتثبيت التلقائي كليهما من الإعدادات.

## مديرو الحزم

يجري تجهيز حزم لـ winget وHomebrew وFlathub وSnap. إلى أن تُنشر، فالملفات أعلاه هي طريقة تثبيت Caruspex.
