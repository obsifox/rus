package rus.skin;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;

/**
 * One agent for Minecraft 1.7 through 26.3.
 * Authlib's class names stay stable; the game client does not.
 */
public final class AuthlibTransformer implements ClassFileTransformer, Opcodes {
    private static final String LEGACY = "skins.minecraft.net";

    @Override
    public byte[] transform(ClassLoader loader, String name, Class<?> type, ProtectionDomain domain, byte[] bytes) {
        if (name == null || bytes == null || name.contains("rus/skin/")) {
            return null;
        }
        try {
            byte[] out = null;
            if (name.contains("mojang/authlib/")) {
                out = transformAuthlib(name, bytes);
            }
            byte[] source = out != null ? out : bytes;
            if (contains(source, LEGACY)) {
                byte[] rewritten = rewriteLegacy(source);
                if (rewritten != null) {
                    out = rewritten;
                }
            }
            return out;
        } catch (Throwable t) {
            Hooks.log("transform " + name + " failed: " + t);
            return null;
        }
    }

    private static byte[] transformAuthlib(String name, byte[] bytes) {
        ClassReader reader = new ClassReader(bytes);
        ClassWriter writer = new ClassWriter(reader, 0);
        boolean[] changed = {false};
        String properties = fieldDesc(reader, "properties");
        boolean profile = name.endsWith("GameProfile");
        reader.accept(new ClassVisitor(ASM9, writer) {
            @Override
            public MethodVisitor visitMethod(int access, String method, String desc, String sig, String[] ex) {
                MethodVisitor next = super.visitMethod(access, method, desc, sig, ex);
                if (next == null || noCode(access)) {
                    return next;
                }
                if (forceTrue(method, desc)) {
                    changed[0] = true;
                    return replaceTrue(next);
                }
                if ("getSecurePropertyValue".equals(method) && desc.endsWith(")Ljava/lang/String;")) {
                    changed[0] = true;
                    return replaceValue(next);
                }
                if ("getPropertySignatureState".equals(method) && desc.contains(")L") && desc.endsWith(";")) {
                    changed[0] = true;
                    return replaceSigned(next, desc.substring(desc.lastIndexOf(')') + 1));
                }
                if (profile && properties != null && isPropertiesGetter(method, desc)) {
                    changed[0] = true;
                    return replaceProperties(next, name, properties);
                }
                return next;
            }
        }, 0);
        return changed[0] ? writer.toByteArray() : null;
    }

    private static boolean noCode(int access) {
        return (access & (ACC_ABSTRACT | ACC_NATIVE)) != 0;
    }

    private static boolean isPropertiesGetter(String method, String desc) {
        if (desc == null || !desc.startsWith("()") || !desc.endsWith("PropertyMap;")) {
            return false;
        }
        return "getProperties".equals(method) || "properties".equals(method);
    }

    private static boolean forceTrue(String name, String desc) {
        if (desc == null || !desc.endsWith(")Z")) {
            return false;
        }
        return "isSignatureValid".equals(name)
            || "validateProperty".equals(name)
            || "isAllowedTextureDomain".equals(name)
            || "isWhitelistedDomain".equals(name)
            || "hasSignature".equals(name);
    }

    private static String fieldDesc(ClassReader reader, String field) {
        String[] found = {null};
        reader.accept(new ClassVisitor(ASM9) {
            @Override
            public org.objectweb.asm.FieldVisitor visitField(int access, String name, String desc, String sig, Object value) {
                if (field.equals(name)) {
                    found[0] = desc;
                }
                return null;
            }
        }, ClassReader.SKIP_CODE);
        return found[0];
    }

    private static MethodVisitor replaceTrue(MethodVisitor delegate) {
        return new MethodVisitor(ASM9) {
            @Override
            public void visitEnd() {
                delegate.visitCode();
                delegate.visitInsn(ICONST_1);
                delegate.visitInsn(IRETURN);
                delegate.visitMaxs(1, 3);
                delegate.visitEnd();
            }
        };
    }

    private static MethodVisitor replaceValue(MethodVisitor delegate) {
        return new MethodVisitor(ASM9) {
            @Override
            public void visitEnd() {
                delegate.visitCode();
                delegate.visitVarInsn(ALOAD, 1);
                delegate.visitMethodInsn(INVOKESTATIC, "rus/skin/Hooks", "propertyValue", "(Ljava/lang/Object;)Ljava/lang/String;", false);
                delegate.visitInsn(ARETURN);
                delegate.visitMaxs(1, 3);
                delegate.visitEnd();
            }
        };
    }

    private static MethodVisitor replaceSigned(MethodVisitor delegate, String returnDesc) {
        Type type = Type.getType(returnDesc);
        return new MethodVisitor(ASM9) {
            @Override
            public void visitEnd() {
                delegate.visitCode();
                delegate.visitLdcInsn(type);
                delegate.visitMethodInsn(INVOKESTATIC, "rus/skin/Hooks", "signed", "(Ljava/lang/Class;)Ljava/lang/Object;", false);
                delegate.visitTypeInsn(CHECKCAST, type.getInternalName());
                delegate.visitInsn(ARETURN);
                delegate.visitMaxs(1, 2);
                delegate.visitEnd();
            }
        };
    }

    private static MethodVisitor replaceProperties(MethodVisitor delegate, String owner, String desc) {
        String internal = Type.getType(desc).getInternalName();
        return new MethodVisitor(ASM9) {
            @Override
            public void visitEnd() {
                delegate.visitCode();
                delegate.visitVarInsn(ALOAD, 0);
                delegate.visitInsn(DUP);
                delegate.visitFieldInsn(GETFIELD, owner, "properties", desc);
                delegate.visitMethodInsn(INVOKESTATIC, "rus/skin/Hooks", "decorate", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;", false);
                delegate.visitTypeInsn(CHECKCAST, internal);
                delegate.visitInsn(ARETURN);
                delegate.visitMaxs(2, 1);
                delegate.visitEnd();
            }
        };
    }

    private static byte[] rewriteLegacy(byte[] bytes) {
        ClassReader reader = new ClassReader(bytes);
        ClassWriter writer = new ClassWriter(reader, 0);
        boolean[] changed = {false};
        reader.accept(new ClassVisitor(ASM9, writer) {
            @Override
            public org.objectweb.asm.FieldVisitor visitField(int access, String name, String desc, String sig, Object value) {
                Object next = rewrite(value, changed);
                return super.visitField(access, name, desc, sig, next);
            }

            @Override
            public MethodVisitor visitMethod(int access, String name, String desc, String sig, String[] ex) {
                MethodVisitor next = super.visitMethod(access, name, desc, sig, ex);
                if (next == null) {
                    return null;
                }
                return new MethodVisitor(ASM9, next) {
                    @Override
                    public void visitLdcInsn(Object value) {
                        super.visitLdcInsn(rewrite(value, changed));
                    }
                };
            }
        }, 0);
        return changed[0] ? writer.toByteArray() : null;
    }

    private static Object rewrite(Object value, boolean[] changed) {
        if (!(value instanceof String)) {
            return value;
        }
        String text = (String) value;
        if (!text.contains(LEGACY)) {
            return value;
        }
        String root = SkinServer.root();
        if (root == null) {
            return value;
        }
        changed[0] = true;
        if (text.contains("%s")) {
            return root + "legacy/%s.png";
        }
        return root + "skin.png";
    }

    private static boolean contains(byte[] bytes, String text) {
        byte[] needle = text.getBytes(java.nio.charset.StandardCharsets.US_ASCII);
        outer:
        for (int i = 0; i + needle.length <= bytes.length; i++) {
            for (int j = 0; j < needle.length; j++) {
                if (bytes[i + j] != needle[j]) {
                    continue outer;
                }
            }
            return true;
        }
        return false;
    }
}
