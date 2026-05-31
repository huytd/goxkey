# ibus 安装

GNOME 桌面环境集成了 ibus 输入法框架:
<https://help.gnome.org/misc/release-notes/3.6/i18n-ibus.html>

所以在 GNOME 中使用 ibus 输入法是比较方便的.


## 安装

操作系统: ArchLinux

需要安装以下软件包:

```
> pacman -Ss ibus

extra/ibus 1.5.29-3 [已安装]
    Next Generation Input Bus for Linux

extra/ibus-libpinyin 1.15.3-1 [已安装]
    Intelligent Pinyin engine based on libpinyin for IBus

extra/libibus 1.5.29-3 [已安装]
    IBus support library
```

所以安装命令为:

```sh
sudo pacman -S ibus ibus-libpinyin
```

TODO
