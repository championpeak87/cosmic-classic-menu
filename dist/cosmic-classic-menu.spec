Name:           cosmic-classic-menu
Version:        1.0.0
Release:        1%{?dist}
Summary:        Cosmic Classic Menu Application

License:        MIT
URL:            https://example.com/cosmic-classic-menu
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
Requires:       some-dependency

%description
Cosmic Classic Menu is a Rust-based application for managing cosmic menus.

%prep
%autosetup

%build
cargo build --release

%install
install -D -m 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%{_bindir}/%{name}

%changelog
* Wed Feb 19 2025 Your Name <your.email@example.com> - 1.0.0-1
- Initial package