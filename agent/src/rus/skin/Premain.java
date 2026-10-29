package rus.skin;

import java.lang.instrument.Instrumentation;

/** Attached by the launcher. Not a mod, and not a resource pack. */
public final class Premain {
    public static void premain(String args, Instrumentation inst) {
        start(inst);
    }

    public static void agentmain(String args, Instrumentation inst) {
        start(inst);
    }

    private static void start(Instrumentation inst) {
        try {
            SkinServer.start();
            inst.addTransformer(new AuthlibTransformer(), false);
            Hooks.log("agent ready");
        } catch (Throwable t) {
            Hooks.log("agent failed: " + t);
        }
    }

    private Premain() {}
}
