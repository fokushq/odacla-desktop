# Odacla CI/CD Sürüm (Release) Kılavuzu

Bu proje, yeni bir versiyon etiketi (tag) atıldığında Windows (.msi), Ubuntu (.deb) ve macOS (.dmg, universal — Apple Silicon + Intel) kurulum dosyalarını otomatik olarak derleyen bir GitHub Actions altyapısına sahiptir.

## Nasıl Yeni Sürüm Yayınlanır?

**1. Versiyon numaralarını güncelleyin** (tag ile aynı olmalı, yoksa release adı tag'den farklı düşer):

- `apps/desktop/src-tauri/tauri.conf.json` → `"version"`
- `Cargo.toml` (workspace) → `[workspace.package] version`
- `apps/desktop/package.json` → `"version"`

**2. Kodunuzu commit'leyin ve gönderin:**

```bash
git add .
git commit -m "chore: bump version to 1.0.1"
git push origin main
```

**3. Versiyon etiketini oluşturun ve gönderin:**

```bash
git tag v1.0.1
git push origin v1.0.1
```

Tag'i gönderdiğiniz an GitHub Actions devreye girer; yaklaşık 10–15 dakika içinde projenin **Releases** sekmesinde üç platformun kurulum dosyaları otomatik olarak belirir.

## Notlar

- Workflow yalnızca `v*` kalıbındaki tag'lerde tetiklenir; normal branch push'ları build başlatmaz.
- Ekstra secret gerekmez — `GITHUB_TOKEN` otomatik sağlanır.
- macOS build'i imzasızdır (code signing yok); kullanıcıların ilk açılışta sağ tık → Aç yapması gerekebilir. İleride Apple Developer sertifikasıyla imzalama eklenebilir.
