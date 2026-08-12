<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.png">
  <img src="assets/banner-light.png" width="100%" alt="Talkdedsec Visual — Windows için gerçek zamanlı ekran renk motoru.">
</picture>

<p align="center">
  <a href="https://github.com/Talkdedsec1/talkdedsec-visual/releases/latest"><b>indir</b></a>
  &nbsp;·&nbsp;
  <a href="#kaynaktan-derleme"><b>derle</b></a>
  &nbsp;·&nbsp;
  <a href="#nasıl-çalışıyor"><b>nasıl çalışıyor</b></a>
  &nbsp;·&nbsp;
  <a href="README.md"><b>English</b></a>
</p>

<br>

## Bu ne

Tüm ekran için bir renk paneli. Beş slider — parlaklık, kontrast, doygunluk, renk tonu ve gece görüşü —
tek bir 5×5 renk matrisini besliyor, o matris de doğrudan Windows'a veriliyor. Masaüstünün birleştirdiği
her piksel oradan geçiyor: oyunlar, video, tarayıcı, hepsi.

Bu bir oyun modu değil. Oyun klasörüne dosya kopyalanmıyor, hiçbir sürece bağlanılmıyor, kütüphane
enjekte edilmiyor, sürücü kurulmuyor. Program Windows'tan ekranı renklendirmesini istiyor, Windows da
yapıyor — Ayarlar'daki yerleşik renk filtrelerinin çalıştığı yolun aynısı. Yönetici yetkisi istememesinin
ve kare süresinde ölçülebilir bir maliyeti olmamasının sebebi de bu.

<br>

## Panel

<img src="assets/screenshot.png" width="100%" alt="Talkdedsec Visual paneli: preset rayı, üç kontrol kartı, önce/sonra bölmeli canlı önizleme ve profil rayı">

Sol rayda presetler duruyor; küçük resimler ekran görüntüsü değil, aynı sahnenin o presetin gerçek
matrisinden geçirilmiş hali — yani kartta gördüğün şey presetin yaptığı şey. Orta sütunda üç kontrol
kartı. Altında sürüklenebilir önce/sonra bölmeli canlı önizleme: sliderları orada ayarlayıp sonucu ekrana
hiç dokunmadan görüyorsun. **Basılı tut: orijinal** düğmesi, basılı tuttuğun sürece filtreyi kaldırıyor.

Sağ ray profilleri kaydediyor ve canlı matrisi gösteriyor: motorun o an uyguladığı on iki katsayı,
sürükledikçe güncelleniyor.

<br>

## Kontroller

| Kontrol | Aralık | Nötr | Ne yapıyor |
|---|---|---|---|
| Parlaklık | −0,50 … +0,50 | 0,00 | Siyah seviyesini kaldırıyor ya da düşürüyor |
| Kontrast | 0,50 … 2,00 | 1,00 | Orta griyi merkez alıp gölge–ışık aralığını açıyor |
| Doygunluk | 0,00 … 3,00 | 1,00 | Renk yoğunluğu; griden aşırı doyguna |
| Renk Tonu | −180° … +180° | 0° | Tüm paleti döndürüyor |
| Gece Görüşü | 0,00 … 1,00 | 0,00 | Gölge detayını siyaha kırpmak yerine kaldırıp yeşile çekiyor |

Her sliderda nötr konumu gösteren bir çizgi var ve her değer kutusu yazılabilir — `1,52` yaz,
<kbd>Enter</kbd>'a bas, tamam.

<br>

## Nasıl çalışıyor

Beş kontrol tek bir afin matriste birleşiyor ve tek geçişte uygulanıyor:

```
kontrast → doygunluk → renk tonu → gece görüşü → parlaklık
```

Birleştirme düpedüz matris çarpımı, dolayısıyla sıra sabit ve sonuç her değişiklikte tek bir
`MagSetFullscreenColorEffect` çağrısı. Matris matematiği [`src/color.rs`](src/color.rs) içinde ve birim
testleriyle korunuyor: sıfır doygunluk Rec.709 parlaklığına eşit olmalı, kontrast tam orta gri üzerinde
dönmeli, 360° ton dönüşü birim matrisi vermeli ve beşini birleştirmek tek tek uygulamakla aynı sonucu
üretmeli.

```bash
cargo test
```

<br>

## Kurulum

[Releases](https://github.com/Talkdedsec1/talkdedsec-visual/releases/latest) sayfasından
`talkdedsec-visual.exe` dosyasını indir ve çalıştır. Tek dosya; kurulum yok, .NET yok, WebView2 yok,
hiçbir runtime yok. Dosya henüz imzalı olmadığı için Windows SmartScreen ilk seferinde uyarı verecek;
**Ek bilgi → Yine de çalıştır** demeden önce aşağıdaki özeti doğrula.

Ayarlar, profiller ve son slider konumları tek bir dosyada:

```
%APPDATA%\Talkdedsec\Visual\config.json
```

Sil, program sıfırdan açılır. Başka hiçbir yere bir şey yazılmıyor ve hiçbir yere bir şey gönderilmiyor —
program hiç soket açmıyor.

### İndirmeyi doğrula

`talkdedsec-visual.exe` için SHA-256, `v0.1.0` sürümü:

```text
a0b2fa66984b46d5a7f3003a8d2e10d38f3cfe6d8881e57b5cca40f2a89bc925
```

```powershell
Get-FileHash .\talkdedsec-visual.exe -Algorithm SHA256
```

<br>

## Tepside yaşamak

Pencereyi kapatmak programı sonlandırmıyor, tepsiye indiriyor — böylece oynarken filtre açık kalıyor.
Tepsi menüsü pencereyi geri getiriyor, filtreyi açıp kapatıyor ya da programı gerçekten kapatıyor.
Çarpı tuşunun gerçekten kapatmasını istersen ayarlardan kapatabilirsin.

Global kısayol — <kbd>F6</kbd>–<kbd>F12</kbd> arası, varsayılan <kbd>F9</kbd> — oyundan çıkmadan
filtreyi açıp kapatıyor. Tuş başka bir program tarafından kullanılıyorsa panel sessizce başarısız olmak
yerine bunu söylüyor.

Windows açılışında başlatma, `HKCU\...\CurrentVersion\Run` altında tek bir kayıt değeri; kapatınca
değer siliniyor.

<br>

## Profiller

Anlık slider konumlarını isimlendirdiğinde kaydediliyor. Var olan bir isme kaydetmek üzerine yazıyor,
yani tekrar tekrar kaydetmek kopya yığmıyor. Profiller düz JSON olarak dışa aktarılıp geri alınabiliyor;
makineler arası taşımanın yolu da bu:

```json
[
  {
    "name": "gece",
    "settings": {
      "brightness": 0.08,
      "contrast": 1.1,
      "saturation": 0.6,
      "hue": 0.0,
      "night_vision": 0.85
    }
  }
]
```

<br>

## Kaynaktan derleme

```bash
git clone https://github.com/Talkdedsec1/talkdedsec-visual
cd talkdedsec-visual
cargo build --release
```

Tek gereksinim Rust 1.85 ve üzeri. C++ toolchain adımı yok, Python yok, `node_modules` yok.
Çıktı `target/release/talkdedsec-visual.exe`, yaklaşık 9,4 MB.

| Yol | İçinde ne var |
|---|---|
| `src/color.rs` | 5×5 matris matematiği ve testleri |
| `src/engine.rs` | Magnification API bağlantısı |
| `src/preview.rs` | Prosedürel önizleme sahnesi |
| `src/presets.rs` | Hazır presetler |
| `src/profiles.rs` | Profil deposu ve JSON içe/dışa aktarma |
| `src/system.rs` | Tepsi, global kısayol, açılışta başlatma |
| `ui/` | Slint arayüzü: `main`, `widgets`, `icons`, `theme` |

`preview.rs` içindeki önizleme sahnesi çekilmiş değil, üretilmiş: gökyüzü geçişi, ağaç hattı, arazi,
gece görüşünün üzerinde çalışabileceği bilinçli olarak karanlık bir cep ve on iki kareli kalibrasyon
şeridi. Bu depoda kimsenin görselinden izlenmiş hiçbir şey yok.

<br>

## Oyunlar hakkında

Bu araç ekranın gözüne ne gönderdiğini değiştiriyor, oyunun ekrana ne gönderdiğini değil. Bu gerçek bir
ayrım ve buradaki hiçbir şeyin anti-cheat'e takılmamasının sebebi de bu.

Ama bir garanti değil. Bazı rekabetçi oyunlar görüşü iyileştiren harici görüntü ayarlarına izin vermiyor
ve bu teknik bir soru değil, onların kararı. Oynadığın oyunun kurallarını oku ve kendin karar ver.

<br>

## Lisans

[GPL-3.0-or-later](LICENSE) — © 2026 Talkdedsec

Al, değiştir, dağıt. Türev iş de açık kalmak zorunda.
