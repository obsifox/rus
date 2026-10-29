package rus.skin;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

/** Serves the imported skin and cape on localhost. Daemon-only, so the game can exit. */
public final class SkinServer {
    private static volatile String base;

    static void start() throws Exception {
        if (base != null) {
            return;
        }
        ServerSocket socket = new ServerSocket(0, 8, java.net.InetAddress.getByName("127.0.0.1"));
        base = "http://127.0.0.1:" + socket.getLocalPort() + "/";
        Thread thread = new Thread(() -> acceptLoop(socket), "rus-skin");
        thread.setDaemon(true);
        thread.start();
        Hooks.log("serving " + base);
    }

    static String root() {
        return base;
    }

    static String url(String name) {
        String root = base;
        if (root == null) {
            return null;
        }
        String path = System.getProperty(name.startsWith("cape") ? "rus.cape" : "rus.skin", "").trim();
        if (path.isEmpty() || !Files.isRegularFile(Paths.get(path))) {
            return null;
        }
        return root + name;
    }

    private static void acceptLoop(ServerSocket socket) {
        while (!socket.isClosed()) {
            try {
                Socket client = socket.accept();
                Thread worker = new Thread(() -> handle(client), "rus-skin-io");
                worker.setDaemon(true);
                worker.start();
            } catch (Exception e) {
                if (!socket.isClosed()) {
                    Hooks.log("accept failed: " + e);
                }
                return;
            }
        }
    }

    private static void handle(Socket client) {
        try (Socket socket = client) {
            socket.setSoTimeout(2000);
            InputStream in = socket.getInputStream();
            ByteArrayOutputStream head = new ByteArrayOutputStream();
            while (head.size() < 1024) {
                int b = in.read();
                if (b < 0) {
                    break;
                }
                head.write(b);
                if (head.size() >= 4) {
                    byte[] raw = head.toByteArray();
                    int n = raw.length;
                    if (n >= 4 && raw[n - 4] == '\r' && raw[n - 3] == '\n' && raw[n - 2] == '\r' && raw[n - 1] == '\n') {
                        break;
                    }
                }
            }
            String request = head.toString("ISO-8859-1");
            String target = "/";
            int space = request.indexOf(' ');
            int next = request.indexOf(' ', space + 1);
            if (space > 0 && next > space) {
                target = request.substring(space + 1, next);
            }
            int query = target.indexOf('?');
            if (query >= 0) {
                target = target.substring(0, query);
            }
            byte[] body = new byte[0];
            int code = 404;
            String file = fileFor(target);
            if (file != null) {
                body = Files.readAllBytes(Paths.get(file));
                code = 200;
            }
            String status = code == 200 ? "OK" : "Not Found";
            byte[] header = ("HTTP/1.1 " + code + " " + status + "\r\nContent-Type: image/png\r\nContent-Length: " + body.length + "\r\nConnection: close\r\n\r\n").getBytes(StandardCharsets.US_ASCII);
            OutputStream out = socket.getOutputStream();
            out.write(header);
            out.write(body);
            out.flush();
        } catch (Exception ignored) {
        }
    }

    private static String fileFor(String target) {
        if (target.startsWith("/cape.png")) {
            return existing(System.getProperty("rus.cape", ""));
        }
        if (target.startsWith("/skin.png")) {
            return existing(System.getProperty("rus.skin", ""));
        }
        if (target.startsWith("/legacy/")) {
            String name = target.substring("/legacy/".length());
            int slash = name.lastIndexOf('/');
            if (slash >= 0) {
                name = name.substring(slash + 1);
            }
            if (name.endsWith(".png")) {
                name = name.substring(0, name.length() - 4);
            }
            String want = System.getProperty("rus.name", "Player").trim();
            if (!name.equalsIgnoreCase(want)) {
                return null;
            }
            return existing(System.getProperty("rus.skin", ""));
        }
        return null;
    }

    private static String existing(String path) {
        if (path == null) {
            return null;
        }
        String trimmed = path.trim();
        if (trimmed.isEmpty() || !Files.isRegularFile(Paths.get(trimmed))) {
            return null;
        }
        return trimmed;
    }

    private SkinServer() {}
}
