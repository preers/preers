<template>
  <div class="sidebar">
    <h2>Preers</h2>
    <ul>
      <li @click="selectItem('Peers')">Network Information</li>
      <li @click="selectItem('Rendezvous')">Rendezvous</li>
      <li @click="selectItem('ProvideServices')">Provide Services</li>
      <li @click="selectItem('UseServices')">Use Services</li>
    </ul>
  </div>
</template>

<script setup>
import { defineEmits } from 'vue';

const emits = defineEmits(['selectItem']);

function selectItem(item) {
  emits('selectItem', item);
}
</script>

<style scoped>
.sidebar {
  width: 200px;
  position: fixed;
  left: 0;
  top: 0;
  bottom: 0;
  background-color: #f4f4f4;
  overflow-y: auto;
  box-shadow: 2px 0 5px rgba(0,0,0,0.1);
}

h2 {
  margin: 20px;
  text-align: center;
}

ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

li {
  cursor: pointer;
  padding: 10px;
  border-bottom: 1px solid #ccc;
}

li:hover {
  background-color: #eaeaea;
}
</style>