<template>
  <section class="profil" aria-labelledby="profil-titre">
    <h2 id="profil-titre">Mon profil</h2>

    <form class="profil__form" @submit.prevent="enregistrer">
      <label for="nom">Nom</label>
      <input id="nom" v-model="nom" type="text" />

      <label for="courriel">Courriel</label>
      <input id="courriel" v-model="courriel" type="email" />

      <input type="tel" v-model="telephone" placeholder="Téléphone" />

      <textarea v-model="bio"></textarea>

      <select v-model="langue" aria-label="Langue de l'interface">
        <option value="fr">Français</option>
        <option value="en">English</option>
      </select>

      <img :src="avatarUrl" :alt="avatarLegende" class="profil__avatar" />
      <img src="/img/separateur.png" />
      <img src="/img/decoratif.png" alt="" />

      <button type="submit">Enregistrer</button>
      <button type="button" @click="annuler"></button>
      <button type="button" @click="supprimer" aria-label="Supprimer le compte">
        <img src="/icons/trash.svg" alt="" />
      </button>
    </form>

    <nav>
      <a href="/aide">Aide</a>
      <a :href="lienExport">{{ libelleExport }}</a>
      <a href="/deconnexion"><img src="/icons/exit.svg" /></a>
    </nav>
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";

const nom = ref("");
const courriel = ref("");
const telephone = ref("");
const bio = ref("");
const langue = ref("fr");
const avatarUrl = ref("/img/avatar.png");
const avatarLegende = ref("Photo de profil");
const lienExport = ref("/export");
const libelleExport = ref("Exporter mes données");

function enregistrer() {}
function annuler() {}
function supprimer() {}
</script>

<style scoped>
.profil__avatar {
  width: 96px;
}
/* a < b is not a tag, and neither is this: <img> */
</style>
