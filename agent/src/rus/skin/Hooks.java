package rus.skin;

import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Collection;
import java.util.Map;
import java.util.UUID;

/** Puts the launcher's chosen skin and cape onto the local profile. */
public final class Hooks {
    private static volatile UUID localId;
    private static volatile String texturesValue;
    private static volatile boolean logged;

    /** Called from GameProfile.getProperties / properties. Returns the same map, or a copy that has our textures. */
    public static Object decorate(Object profile, Object map) {
        if (profile == null || map == null || !isLocal(profile)) {
            return map;
        }
        String value = texturesValue();
        if (value == null) {
            return map;
        }
        try {
            if (already(map, value)) {
                return map;
            }
            Object property = newProperty(map, value);
            try {
                invoke(map, "removeAll", "textures");
                invoke(map, "put", "textures", property);
                logOnce(profile);
                return map;
            } catch (Throwable ignored) {
                Object copy = copyWith(map, property);
                logOnce(profile);
                return copy;
            }
        } catch (Throwable t) {
            log("decorate failed: " + t);
            return map;
        }
    }

    public static boolean isLocal(Object profile) {
        UUID want = localId();
        if (want == null || profile == null) {
            return false;
        }
        for (String method : new String[] {"getId", "id"}) {
            try {
                Object value = profile.getClass().getMethod(method).invoke(profile);
                if (value instanceof UUID && want.equals(value)) {
                    return true;
                }
                if (value instanceof String && sameText(want, (String) value)) {
                    return true;
                }
            } catch (Throwable ignored) {
            }
        }
        return false;
    }

    /** Newer clients refuse a texture unless its signature state is SIGNED. */
    public static Object signed(Class<?> type) {
        return Enum.valueOf(type.asSubclass(Enum.class), "SIGNED");
    }

    public static String propertyValue(Object property) {
        if (property == null) {
            return "";
        }
        try {
            return (String) property.getClass().getMethod("getValue").invoke(property);
        } catch (Throwable ignored) {
        }
        try {
            return (String) property.getClass().getMethod("value").invoke(property);
        } catch (Throwable t) {
            return "";
        }
    }

    private static Object newProperty(Object map, String value) throws Exception {
        Class<?> type = Class.forName(propertyName(map), true, map.getClass().getClassLoader());
        try {
            return type.getConstructor(String.class, String.class, String.class).newInstance("textures", value, "rus");
        } catch (NoSuchMethodException e) {
            return type.getConstructor(String.class, String.class).newInstance("textures", value);
        }
    }

    private static Object copyWith(Object map, Object property) throws Exception {
        ClassLoader loader = map.getClass().getClassLoader();
        Object multi = null;
        for (String name : new String[] {
            "com.google.common.collect.HashMultimap",
            "com.google.common.collect.ArrayListMultimap"
        }) {
            try {
                multi = Class.forName(name, true, loader).getMethod("create").invoke(null);
                break;
            } catch (Throwable ignored) {
            }
        }
        if (multi == null) {
            throw new IllegalStateException("no multimap");
        }
        Method put = multi.getClass().getMethod("put", Object.class, Object.class);
        Object entries = map.getClass().getMethod("entries").invoke(map);
        if (entries instanceof Iterable) {
            for (Object entry : (Iterable<?>) entries) {
                Map.Entry<?, ?> pair = (Map.Entry<?, ?>) entry;
                if (!"textures".equals(pair.getKey())) {
                    put.invoke(multi, pair.getKey(), pair.getValue());
                }
            }
        }
        put.invoke(multi, "textures", property);
        for (Constructor<?> ctor : map.getClass().getConstructors()) {
            if (ctor.getParameterCount() == 1) {
                return ctor.newInstance(multi);
            }
        }
        for (Constructor<?> ctor : map.getClass().getConstructors()) {
            if (ctor.getParameterCount() == 0) {
                Object fresh = ctor.newInstance();
                invoke(fresh, "put", "textures", property);
                return fresh;
            }
        }
        throw new IllegalStateException("no PropertyMap constructor");
    }

    private static Object invoke(Object target, String name, Object... args) throws Exception {
        Class<?>[] types = new Class<?>[args.length];
        for (int i = 0; i < args.length; i++) {
            types[i] = Object.class;
        }
        return target.getClass().getMethod(name, types).invoke(target, args);
    }

    private static boolean already(Object map, String value) throws Exception {
        Object got = map.getClass().getMethod("get", Object.class).invoke(map, "textures");
        if (!(got instanceof Collection)) {
            return false;
        }
        Collection<?> list = (Collection<?>) got;
        if (list.isEmpty()) {
            return false;
        }
        return value.equals(propertyValue(list.iterator().next()));
    }

    private static String propertyName(Object map) {
        String name = map.getClass().getName();
        int cut = name.indexOf("properties.PropertyMap");
        if (cut < 0) {
            return "com.mojang.authlib.properties.Property";
        }
        return name.substring(0, cut) + "properties.Property";
    }

    private static UUID localId() {
        UUID cached = localId;
        if (cached != null) {
            return cached;
        }
        String raw = System.getProperty("rus.uuid", "").trim();
        if (raw.isEmpty()) {
            return null;
        }
        try {
            cached = UUID.fromString(raw);
            localId = cached;
            return cached;
        } catch (IllegalArgumentException e) {
            log("bad uuid " + raw);
            return null;
        }
    }

    private static boolean sameText(UUID want, String raw) {
        String text = raw == null ? "" : raw.trim();
        return text.equalsIgnoreCase(want.toString()) || text.equalsIgnoreCase(want.toString().replace("-", ""));
    }

    static String texturesValue() {
        String cached = texturesValue;
        if (cached != null) {
            return cached;
        }
        String skin = System.getProperty("rus.skin", "").trim();
        if (skin.isEmpty() || SkinServer.url("skin.png") == null) {
            return null;
        }
        String cape = System.getProperty("rus.cape", "").trim();
        String model = System.getProperty("rus.model", "").trim();
        boolean slim = "slim".equalsIgnoreCase(model) || "alex".equalsIgnoreCase(model);
        String name = json(System.getProperty("rus.name", "Player"));
        String id = System.getProperty("rus.uuid", "").replace("-", "");
        StringBuilder textures = new StringBuilder();
        textures.append("{\"timestamp\":").append(System.currentTimeMillis());
        textures.append(",\"profileId\":\"").append(json(id)).append('"');
        textures.append(",\"profileName\":\"").append(name).append('"');
        textures.append(",\"isPublic\":true");
        textures.append(",\"textures\":{\"SKIN\":{\"url\":\"").append(json(SkinServer.url("skin.png"))).append('"');
        if (slim) {
            textures.append(",\"metadata\":{\"model\":\"slim\"}");
        }
        textures.append('}');
        if (!cape.isEmpty() && SkinServer.url("cape.png") != null) {
            textures.append(",\"CAPE\":{\"url\":\"").append(json(SkinServer.url("cape.png"))).append("\"}");
        }
        textures.append("}}");
        cached = Base64.getEncoder().encodeToString(textures.toString().getBytes(StandardCharsets.UTF_8));
        texturesValue = cached;
        return cached;
    }

    private static void logOnce(Object profile) {
        if (!logged) {
            logged = true;
            log("applied imported skin/cape to " + profile);
        }
    }

    private static String json(String raw) {
        if (raw == null) {
            return "";
        }
        return raw.replace("\\", "\\\\").replace("\"", "\\\"");
    }

    static void log(String line) {
        String path = System.getProperty("rus.log", "").trim();
        String text = "[rus-skin] " + line;
        System.err.println(text);
        if (path.isEmpty()) {
            return;
        }
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                (text + System.lineSeparator()).getBytes(StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (Throwable ignored) {
        }
    }

    private Hooks() {}
}
