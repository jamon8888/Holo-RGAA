<template>
  <section class="profile-card">
    <img class="avatar" src="/img/avatar-default.png" alt="Photo de profil" />
    <a :href="profileUrl" aria-label="Ouvrir le profil complet">Voir</a>
  </section>
</template>

<script setup>
defineProps({ profileUrl: String });
</script>
