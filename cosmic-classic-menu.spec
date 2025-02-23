Name:           cosmic-classic-menu
Version:        0.0.2
Release:        1%{?dist}
Summary:        Cosmic Classic Menu Application

License:        MIT
URL:            https://example.com/cosmic-classic-menu
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  rust-xkbcommon-devel
Requires:       cosmic-osd

%description
Cosmic Classic Menu is a Rust-based application for managing cosmic menus.

%prep
%autosetup

%build
cargo build --release
strip target/release/%{name}

%install
install -D -m 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
cp data/com.championpeak87.cosmic-classic-menu.desktop /usr/share/applications

%files
%{_bindir}/%{name}

%changelog
* Sun Feb 23 2025 Kamil Lihan <k.lihan@outlook.com> 0.0.2-1
- Initial preview version

* Wed Feb 19 2025 Kamil Lihan <k.lihan@outlook.com> - 0.0.1
- Initial package
