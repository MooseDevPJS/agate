package agate.cape;

import com.mojang.blaze3d.platform.NativeImage;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.UUID;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.core.ClientAsset;
import net.minecraft.resources.Identifier;

/**
 * The custom cape chosen in the Agate launcher, shown only on your own player.
 * The launcher writes it to config/agate/cape.png before starting the game.
 */
public final class AgateCape {
    private static final Identifier ID = Identifier.fromNamespaceAndPath("agate", "cape/custom");
    private static boolean tried;
    private static ClientAsset.Texture texture;

    private AgateCape() {}

    /** The custom cape for this player, or null to keep what the game chose. */
    public static ClientAsset.Texture forPlayer(UUID player) {
        Minecraft mc = Minecraft.getInstance();
        if (mc == null || mc.getUser() == null || !player.equals(mc.getUser().getProfileId())) {
            return null;
        }
        // Textures can only be created on the render thread; skins are asked for there.
        if (!tried && mc.isSameThread()) {
            tried = true;
            texture = load(mc);
        }
        return texture;
    }

    private static ClientAsset.Texture load(Minecraft mc) {
        Path file = FabricLoader.getInstance().getConfigDir().resolve("agate").resolve("cape.png");
        if (!Files.isRegularFile(file)) {
            return null;
        }
        try (InputStream in = Files.newInputStream(file)) {
            NativeImage image = NativeImage.read(in);
            mc.getTextureManager().register(ID, new DynamicTexture(() -> "Agate custom cape", image));
            System.out.println("[Agate Cape] Custom cape active (" + image.getWidth() + "x" + image.getHeight() + ")");
            return new ClientAsset.ResourceTexture(ID, ID);
        } catch (Exception e) {
            System.err.println("[Agate Cape] Could not load " + file + ": " + e);
            return null;
        }
    }
}
