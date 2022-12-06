# Converter
Converter is a GTK4+libadwaita application that allows you to convert and manipulate a given image. It is a front-end for [ImageMagick](https://github.com/ImageMagick/ImageMagick).

<div align="center">
  <img src="data/screenshots/0.png">
</div>

## Installation
<a href='https://flathub.org/apps/details/io.gitlab.adhami3310.Converter'><img width='240' alt='Download on Flathub' src='https://flathub.org/assets/badges/flathub-badge-en.png'/></a>

## Features

Converter supports converting from the following datatypes:
 - image/jpeg
 - image/png
 - image/webp
 - image/svg+xml (with scaling)
 - image/heif and image/heic
 - image/bmp

Into the following datatypes:
 - image/jpeg
 - image/png
 - image/webp
 - application/pdf
 - image/heif and image/heic
 - image/bmp

It also supports the following options:
 - Changing quality value of lossy compression.
 - Changing the color value of the alpha layer.
 - Changing the DPI of SVG images.
 - Scaleing and resizing the image to given resolution or ratio.

ImageMagick supports many other datatypes. I will add more and possibly even add more options. If you want me to make something of a higher priority please start an issue.

## Contributing
Issues and merge requests are more than welcome. However, please take the following into consideration:

- This project follows the [GNOME Code of Conduct](https://wiki.gnome.org/Foundation/CodeOfConduct)
- Only Flatpak is supported

## Development

### GNOME Builder
The recommended method is to use GNOME Builder:

1. Install [GNOME Builder](https://apps.gnome.org/app/org.gnome.Builder/) from Flathub
1. Open Builder and select "Clone Repository..."
1. Clone `https://gitlab.com/adhami3310/Converter.git` (or your fork)
1. Press "Run Project" (▶) at the top, or `Ctrl`+`Shift`+`[Spacebar]`.

### Flatpak
You can install Converter from the latest commit:

1. Install [`org.flatpak.Builder`](https://github.com/flathub/org.flatpak.Builder) from Flathub
1. Clone `https://gitlab.com/adhami3310/Converter.git` (or your fork)
1. Run `flatpak run org.flatpak.Builder --install --install-deps-from=flathub --default-branch=master --force-clean build-dir io.gitlab.adhami3310.Converter.json` in the terminal from the root of the repository (use `--user` if necessary)

### Meson
You can build and install on your host system by directly using the Meson buildsystem:

1. Install `blueprint-compiler`
1. Run the following commands (with `/usr` prefix):
```
meson --prefix=/usr build
ninja -C build
sudo ninja -C build install
```

