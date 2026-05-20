import socket


def check(host, port):
    try:
        s = socket.socket(
            socket.AF_INET6 if ":" in host else socket.AF_INET, socket.SOCK_STREAM
        )
        s.bind((host, port))
        s.close()
        return True
    except Exception as e:
        return str(e)


print("IPv4 127.0.0.1 5173:", check("127.0.0.1", 5173))
print("IPv6 ::1 5173:", check("::1", 5173))
print("IPv4 0.0.0.0 5173:", check("0.0.0.0", 5173))
print("IPv6 :: 5173:", check("::", 5173))
