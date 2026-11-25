Name:           ghostpath-vpn
Version:        0.1.0
Release:        1%{?dist}
Summary:        GhostPath VPN - Linux VPN client with reliable TUN mode

License:        MIT OR Apache-2.0
URL:            https://github.com/cybermaz/ghost_path
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  openssl-devel
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  libappindicator-gtk3-devel
BuildRequires:  pkgconfig
BuildRequires:  systemd-rpm-macros

Requires:       iproute
Requires:       iptables
Requires:       systemd
Requires:       webkit2gtk4.1
Requires:       libappindicator-gtk3
Requires:       geoclue2

%description
GhostPath VPN provides censorship-resistant tunneling with reliable TUN mode
on Linux, supporting AmneziaVPN (vpn://, .conf), AmneziaWG obfuscation,
VLESS/Reality, VMess, Trojan, Shadowsocks, and WireGuard protocols
without routing loops, DNS leaks, or conflicts with other VPN clients.

%prep
%autosetup

%build
cargo build --release --workspace

%install
rm -rf $RPM_BUILD_ROOT
install -d -m 0755 %{buildroot}%{_bindir}
install -m 0755 target/release/ghostpath %{buildroot}%{_bindir}/ghostpath
install -m 0755 target/release/ghostpathd %{buildroot}%{_bindir}/ghostpathd
install -m 0755 target/release/ghostpath-gui %{buildroot}%{_bindir}/ghostpath-gui
if [ -f packaging/bin/amneziawg-go ]; then
    install -m 0755 packaging/bin/amneziawg-go %{buildroot}%{_bindir}/amneziawg-go
fi

install -d -m 0755 %{buildroot}%{_unitdir}
install -m 0644 systemd/ghostpathd.service %{buildroot}%{_unitdir}/ghostpathd.service

install -d -m 0755 %{buildroot}%{_datadir}/applications
install -m 0644 packaging/ghostpath.desktop %{buildroot}%{_datadir}/applications/ghostpath.desktop

install -d -m 0755 %{buildroot}%{_datadir}/metainfo
install -m 0644 packaging/ghostpath.metainfo.xml %{buildroot}%{_datadir}/metainfo/com.ghostpath.vpn.metainfo.xml

for size in 16 24 32 48 64 128 256 512; do
    install -d -m 0755 %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps
    install -m 0644 packaging/icons/${size}x${size}.png %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/ghostpath.png
done

install -d -m 0755 %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 packaging/ghostpath.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/ghostpath.svg

%pre
getent group ghostpath >/dev/null || groupadd -r ghostpath

%post
%systemd_post ghostpathd.service

%preun
%systemd_preun ghostpathd.service
if [ $1 -eq 0 ]; then
    %{_bindir}/ghostpathd --recover >/dev/null 2>&1 || :
fi

%postun
%systemd_postun_with_restart ghostpathd.service

%files
%{_bindir}/ghostpath
%{_bindir}/ghostpathd
%{_bindir}/ghostpath-gui
%{_unitdir}/ghostpathd.service
%{_datadir}/applications/ghostpath.desktop
%{_datadir}/metainfo/com.ghostpath.vpn.metainfo.xml
%{_datadir}/icons/hicolor/*/apps/ghostpath.png
%{_datadir}/icons/hicolor/scalable/apps/ghostpath.svg

%changelog
* Sun Nov 23 2026 GhostPath Team <dev@ghostpath.vpn> - 0.1.0-1
- Initial release of GhostPath VPN desktop application with Tauri 2
