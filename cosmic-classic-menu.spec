Name:           cosmic-classic-menu
Version:        0.0.2
Release:        1%{?dist}
Summary:        COSMIC Classic Menu applet

License:        GPLv2
URL:            https://github.com/championpeak87/cosmic-classic-menu.git
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  rust-xkbcommon-devel
Requires:       cosmic-osd

%description
COSMIC Classic Menu is a Rust-based applet for COSMIC Desktop

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
/usr/share/applications/com.championpeak87.cosmic-classic-menu.desktop
