<script lang="ts">
  // Visuel plein cadre d'une appli (apporté par un mod via contributes.banners), posé sous le texte du héros.
  let { src }: { src: string } = $props();
</script>

<div class="shot" style:background-image="url('{src}')"></div>

<style>
  .shot { position: absolute; inset: 0; z-index: 0; background-size: cover; background-position: center; background-repeat: no-repeat; }
</style>
