package blocco.cape.mixin;

import blocco.cape.BloccoCape;
import net.minecraft.client.player.AbstractClientPlayer;
import net.minecraft.core.ClientAsset;
import net.minecraft.world.entity.player.PlayerSkin;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Swaps the cape (and the elytra drawn from it) on your own player only. */
@Mixin(AbstractClientPlayer.class)
public abstract class AbstractClientPlayerMixin {
    @Inject(method = "getSkin", at = @At("RETURN"), cancellable = true)
    private void blocco$customCape(CallbackInfoReturnable<PlayerSkin> cir) {
        PlayerSkin skin = cir.getReturnValue();
        if (skin == null) {
            return;
        }
        ClientAsset.Texture cape = BloccoCape.forPlayer(((AbstractClientPlayer) (Object) this).getUUID());
        if (cape != null) {
            cir.setReturnValue(new PlayerSkin(skin.body(), cape, cape, skin.model(), skin.secure()));
        }
    }
}
