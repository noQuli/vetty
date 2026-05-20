# Security Policy

## Scope

Vetty is a security sandbox tool that runs untrusted code inside Firecracker micro-VMs. Security is a core concern of this project.

The following are considered in scope for security reports:

- **VM escape** — Guest code breaking out of the Firecracker VM boundary
- **Host compromise** — Guest code gaining unauthorized access to the host system
- **Daemon vulnerabilities** — Issues in the REST API, WebSocket, or vsock listener that could be exploited
- **Network isolation bypass** — Guest traffic bypassing NAT rules or accessing unintended host services
- **Privilege escalation** — Any path from unprivileged guest code to elevated host privileges

## Reporting a Vulnerability

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please email: **security@vetty.dev**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Any suggested fixes (optional)

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Initial assessment:** Within 1 week
- **Fix or mitigation:** Depends on severity, typically within 30 days

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x (current) | ✅ |

## Known Limitations

Vetty is under active development and has **not been formally audited**. Known limitations:

1. The host network setup uses `iptables` rules managed via `sudo` — misconfiguration could weaken isolation
2. The mitmproxy HTTPS interception requires a CA certificate trusted by the guest, which is baked into the rootfs
3. Serial console access gives interactive shell access to the guest VM

**Do not rely on Vetty as a sole security boundary for adversarial workloads in production environments.**

## Security Best Practices for Users

- Run Firecracker with the recommended [production host setup](https://github.com/firecracker-microvm/firecracker/blob/main/docs/prod-host-setup.md)
- Keep Firecracker updated to the latest stable release
- Use a dedicated user for running Vetty with minimal privileges
- Review iptables rules after Vetty exits to ensure cleanup
