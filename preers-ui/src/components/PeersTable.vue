<template>
  <h3>PeerId: {{ peer_id }}</h3>
  <div class="table-container">
    <h2>Peers</h2>
    <table>
      <thead>
        <tr>
          <th>Peer ID</th>
          <th>Connected</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(peer, index) in peers" :key="index">
          <td>{{ peer.peer_id }}</td>
          <td>{{ peer.connected ? 'True' : 'False' }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted, getCurrentInstance, onUnmounted } from 'vue';
import axios from 'axios';

const peer_id = ref('');
const peers = ref([]);// 存储获取的 peers 数据
const {proxy} = getCurrentInstance();

// 检查数据格式并赋予默认值的函数
function checkAndAssignDefaults(data) {
  if (data && Array.isArray(data.peers)) {
    //console.log(data.peers);
    return {
      peer_id: data.peer_id,
      peers: data.peers.map(peer => ({
        peer_id: peer.peer_id,
        connected: peer.connected,
      })),
    };
  }
  console.error('Data format error: Expected an object with peers array');
  return { peer_Id: '', peers: [] };
}

// 获取网络信息的函数
async function fetchNetworkInfo() {
  try {
    const response = await proxy.$axios.get('/network_info');
    const data = checkAndAssignDefaults(response.data);
    peer_id.value = data.peer_id;
    peers.value = data.peers;
  } catch (error) {
    console.error('Error fetching network info:', error);
  }
}

// 组件加载时获取数据
// onMounted(async () => {
//   try {
//     // console.log("restart peers");
//     const response = await proxy.$axios.get('/network_info'); // 发送 GET 请求
//     const data = checkAndAssignDefaults(response.data);
//     peer_id.value = data.peer_id;
//     peers.value = data.peers;
//   } catch (error) {
//     console.error('Error fetching network info:', error);
//   }
//});

// 在组件挂载时获取数据
onMounted(() => {
  fetchNetworkInfo();
  // 设置定时器，每5秒（5000毫秒）获取一次数据
  const intervalId = setInterval(fetchNetworkInfo, 5000);
  // 在组件卸载时清除定时器
  onUnmounted(() => clearInterval(intervalId));
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