<template>
  <h3>PeerId: {{ peer_id }}</h3>
  <div class="table-container">
    <h2>Peers</h2>
    <table>
      <thead>
        <tr>
          <th>Peer ID</th>
          <th>Connected</th>
          <!-- <th>Action</th> -->
        </tr>
      </thead>
      <tbody>
        <tr v-for="(peer, index) in peers" :key="index">
          <td>{{ peer.peerid }}</td>
          <td>{{ peer.connected ? 'True' : 'False' }}</td>
          <!-- <td>
            <button @click="deletePeer(index)">Delete</button>
          </td> -->
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted, getCurrentInstance } from 'vue';
import axios from 'axios';

const peer_id = ref('');
const peers = ref([]);// 存储获取的 peers 数据
const newPeer = ref({ peerid: '', connected: 'false' });
const {proxy} = getCurrentInstance();

// 检查数据格式并赋予默认值的函数
function checkAndAssignDefaults(data) {
  if (data && Array.isArray(data.peers)) {
    return {
      peer_Id: data.peer_id,
      peers: data.peers.map(peer => ({
        id: peer.id,
        peerid: peer.peerid,
        connected: peer.connected,
      })),
    };
  }
  console.error('Data format error: Expected an object with peers array');
  return { peer_Id: '', peers: [] };
}

// 组件加载时获取数据
onMounted(async () => {
  try {
    console.log("restart peers");
    const response = await proxy.$axios.get('/network_info'); // 发送 GET 请求
    const { peer_Id, peers } = checkAndAssignDefaults(response.data);
    const data = response.data; // 获取响应数据
    peer_id.value = peer_Id;
    peers.value = data.peers; // 将 peers 数据存储到状态中
  } catch (error) {
    console.error('Error fetching network info:', error);
  }
});


</script>

<style scoped>
.table-container {
  text-align: center;
  max-width: 800px;
  margin: auto;
  padding: 20px;
}

table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 20px;
}

th, td {
  padding: 10px;
  border: 1px solid #ddd;
  text-align: left;
}
</style>