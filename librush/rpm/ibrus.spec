Name:       ibrus
Version:    0.2.2
Release:    1%{?dist}
Summary:    ibus module for pmim (a Chinese pinyin input method)
License:    LGPL-2.1-or-later OR GPL-3.0-or-later
URL:        https://github.com/fm-elpac/librush
Requires:   ibus

%description
librush: ibus module for pmim (a Chinese pinyin input method)

%prep
# TODO

%build
# skip

%install
mkdir -p %{buildroot}/usr/lib/pmim
install -Dm755 -t %{buildroot}/usr/lib/pmim %{_topdir}/SOURCES/ibrus
install -Dm644 -t %{buildroot}/usr/share/ibus/component %{_topdir}/SOURCES/pmim_ibrus.xml

%files
/usr/lib/pmim/ibrus
/usr/share/ibus/component/pmim_ibrus.xml

%changelog
# TODO
