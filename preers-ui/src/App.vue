<template>
  <div id="app" class="app-container">
    <Sidebar @selectItem="setCurrentView" />
    <div class="main-content">
      <component :is="currentView" />
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import Sidebar from './components/Sidebar.vue';
import PeersView from './views/PeersView.vue';
import ProvideServicesView from './views/ProvideServicesView.vue';
import UseServicesView from './views/UseServicesView.vue';
import RendezvousView from './views/RendezvousView.vue';

const currentView = ref(PeersView);

function setCurrentView(viewName) {
  switch (viewName) {
    case 'Peers':
      currentView.value = PeersView;
      break;
    case 'ProvideServices':
      currentView.value = ProvideServicesView;
      break;
    case 'UseServices':
      currentView.value = UseServicesView;
      break;
    case 'Rendezvous':
      currentView.value = RendezvousView;
      break;
    default:
      currentView.value = PeersView;
  }
}
</script>

<style>
.app-container {
  display: flex;
  flex-direction: row;
  height: 100vh;
}

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

.main-content {
  flex-grow: 1;
  margin-left: 200px; /* Same as sidebar width */
  padding: 20px;
  overflow-y: auto;
}
</style>