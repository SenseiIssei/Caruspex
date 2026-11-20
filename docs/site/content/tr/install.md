# Kurulum

Her sürüm [GitHub'daki sürümler sayfasında](https://github.com/SenseiIssei/Caruspex/releases/latest) bulunur. Paketler henüz imzalı değildir, bu yüzden Windows ve macOS ilk açılıştan önce bir kez sorar.

## Windows 10 ve 11

| Dosya | Nedir |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Kurulum programı. Yönetici hakları gerektirmez, komut satırı aracı `caruspex_cli`'yi arama yoluna ekleyebilir ve WebView2 çalışma ortamını yalnızca eksikse kurar. |
| `Caruspex-<version>-windows-x64.zip` | Taşınabilir: arşivi açın ve `caruspex.exe`'yi başlatın. |

İlk açılışta Windows SmartScreen uygulamanın tanınmadığını söyleyebilir: "Ek bilgi"ye, ardından "Yine de çalıştır"a tıklayın.

USB adaptörler, USB çipleri için bir sürücüye ihtiyaç duyar. Windows 10 ve 11 bunu genellikle kendisi bulur; bulmazsa [USB üzerinden ELM327](elm327-usb.md) sayfasına bakın.

## macOS 13 ve sonrası (Apple Silicon)

`Caruspex-<version>-macos.dmg` dosyasını indirin, açın ve Caruspex'i Uygulamalar klasörüne sürükleyin. Uygulama henüz noter onaylı olmadığından, ilk seferde sağ tıklayıp "Aç" ile açın.

## Linux (x86_64)

| Dosya | Nedir |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Neredeyse her dağıtımda çalışır: `chmod +x` ile çalıştırılabilir yapın ve başlatın. Kendini de günceller. |
| `Caruspex-<version>-linux-amd64.deb` | Debian ve Ubuntu için: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Seri adaptörleri `sudo` olmadan kullanmak için kendinizi bir kez `dialout` grubuna ekleyin, sonra oturumu kapatıp yeniden açın:

```
sudo usermod -aG dialout $USER
```

## Yalnızca komut satırı aracı

Betikler ve sunucular için arayüzsüz `caruspex_cli` vardır, Windows, Linux ve macOS için. `caruspex_cli help` komutları listeler.

`caruspex_cli report` teşhis raporunu uygulama olmadan bir PDF dosyasına yazar; diğer komutların bildiği her bağlantıyla çalışır (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Rapor Almanca, İngilizce, İspanyolca, Fransızca, Portekizce, İtalyanca, Felemenkçe ve Endonezce olabilir (`--lang`). Türkçe dahil diğer dillerde İngilizce çıkar, çünkü her PDF görüntüleyicide yerleşik olan yazı tiplerinde bu dillerin harfleri yoktur.

## Güncellemeler

Caruspex her kurulumda bir kez güncelleme aranıp aranmayacağını sorar. Yalnızca evet yanıtından sonra GitHub'a daha yeni bir sürüm olup olmadığını sorar, başka hiçbir şey sormaz. Yeni bir sürüm yalnızca imzası uygulamaya gömülü anahtarla eşleşirse kurulur. Kontrol ve otomatik kurulum ayarlardan istediğiniz zaman değiştirilebilir.

## Paket yöneticileri

winget, Homebrew, Flathub ve Snap paketleri hazırlanıyor. Bunlar yayımlanana kadar Caruspex yukarıdaki dosyalarla kurulur.
