# This file is part of BlackArch Linux ( https://www.blackarch.org/ ).
# See COPYING for license details.

pkgname=scrying
pkgver=0.8.2
pkgrel=1
epoch=0
pkgdesc='Collect RDP, web, and VNC screenshots smartly'
groups=('blackarch' 'blackarch-webapp' 'blackarch-recon')
arch=('x86_64')
url='https://github.com/nccgroup/scrying'
license=('GPL3')
makedepends=('cargo')
depends=()
source=("$pkgname::https://github.com/nccgroup/scrying/archive/v$pkgver.tar.gz")
sha512sums=('SKIP')


build() {
  cd "$pkgname-$pkgver"

  cargo build --release
}

package() {
  cd "$pkgname-$pkgver"

  install -Dm 755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm 644 -t "$pkgdir/usr/share/doc/$pkgname/" README.md Changelog.md
  install -Dm 644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
