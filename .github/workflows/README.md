# Fokus CI/CD Sürüm (Release) Kılavuzu

Bu proje, yeni bir sürüm yayınlandığında Windows (.msi) ve Ubuntu (.deb) kurulum dosyalarını otomatik olarak derleyen bir GitHub Actions altyapısına sahiptir.

## Nasıl Yeni Sürüm Yayınlanır?

Geliştirme işlemlerinizi bitirdikten sonra, kodunuzu ana dala (main) göndermeli ve ardından versiyon etiketi (tag) atmalısınız. 

Sırasıyla şu adımları izleyin:

**1. Kodunuzu commit'leyin ve gönderin:**
```bash
git add .
git commit -m "feat: yeni özellikler eklendi"
git push origin main
```

2. Yeni versiyon etiketini oluşturun ve gönderin (Versiyonu güncelleyin):

```bash
git tag v1.0.1
git push origin v1.0.1
```

Bu komutları gönderdiğiniz an GitHub sunucuları devreye girecek ve yaklaşık 10-15 dakika içinde projenin Releases sekmesinde kurulum dosyaları otomatik olarak belirecektir.