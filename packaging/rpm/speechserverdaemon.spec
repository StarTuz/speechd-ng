Name:           speechserverdaemon
Version:        0.2.0
Release:        1%{?dist}
Summary:        Next-generation Linux speech daemon with AI integration

License:        MIT
URL:            https://github.com/StarTuz/speechd-ng
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  alsa-lib-devel
BuildRequires:  dbus-devel

Requires:       alsa-lib
Requires:       dbus
Requires:       python3
Requires:       espeak-ng

%description
SpeechD-NG is a modern, secure, and intelligent speech service designed 
for the Linux ecosystem. It features neural TTS (Piper), AI integration 
(Ollama), wake word detection, voice learning, and Wyoming protocol support.

%prep
%setup -q

%build
cargo build --release

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_libdir}/speechd-ng
mkdir -p %{buildroot}/usr/lib/systemd/user
mkdir -p %{buildroot}%{_docdir}/%{name}

install -m 755 target/release/speechserverdaemon %{buildroot}%{_bindir}/
install -m 755 src/wakeword_bridge.py %{buildroot}%{_libdir}/speechd-ng/
install -m 755 src/wyoming_bridge.py %{buildroot}%{_libdir}/speechd-ng/
install -m 644 systemd/speechd-ng.service %{buildroot}/usr/lib/systemd/user/

%files
%license LICENSE
%doc README.md
%{_bindir}/speechserverdaemon
%{_libdir}/speechd-ng/wakeword_bridge.py
%{_libdir}/speechd-ng/wyoming_bridge.py
/usr/lib/systemd/user/speechd-ng.service

%changelog
* Sat Dec 28 2024 StarTuz <startuz@example.com> - 0.2.0-1
- Release v0.2.0: Complete Phase 14 with full CI hardening
- Added Ping diagnostic method
- Improved offline resilience
- Production-ready packaging
