<template>
  <div class="table-container">
    <h2>Rendezvous</h2>
    <div class="form-container">
      <form @submit.prevent="addRendezvous">
        <!-- <div class="form-row">
          <input v-model="newRendezvous.id" placeholder="ID" required />
        </div> -->
        <div class="form-row">
          <input v-model="newRendezvous.multiaddr" placeholder="Multiaddr" required />
        </div>
        <button type="submit">Add Rendezvous</button>
      </form>
    </div>
    <table>
      <thead>
        <tr>
          <!-- <th>ID</th> -->
          <th>Multiaddr</th>
          <th>Action</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(rendezvous, index) in rendezvousList" :key="index">
          <!-- <td>{{ rendezvous.id }}</td> -->
          <td>{{ rendezvous.multiaddr }}</td>
          <td>
            <button @click="deleteRendezvous(rendezvous)">Delete</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted, getCurrentInstance } from 'vue';
import axios from 'axios';

const rendezvousList = ref([]);
// const newRendezvous = ref({ id: '', multiaddr: '' });
const newRendezvous = ref({ multiaddr: '' });
const {proxy} = getCurrentInstance();

// 检查数据格式并赋予默认值的函数
function checkAndAssignDefaults(data) {
  if (!Array.isArray(data)) {
    console.error('Data format error: Expected an array');
    return [];
  }

  return data.map(item => {
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      console.error('Data format error: Expected an object');
      return {
        id: null, // 可以根据实际情况生成ID或使用其他默认值
        multiaddr: '',
      };
    }

    return {
      id: typeof item.id === 'number' ? item.id : null,
      multiaddr: typeof item.multiaddr === 'string' ? item.multiaddr : '',
    };
  });
}

onMounted(async () => {
  try {
    console.log("restart rez");
    const response = await proxy.$axios.get('/rendezvous');
    rendezvousList.value = checkAndAssignDefaults(response.data);
    // loading.value = false;
  } catch (error) {
    console.error('Error fetching rendezvous:', error);
    // loading.value = false;
  }
});

async function addRendezvous() {
  try {
    const info = JSON.stringify({
      id: 0,
      multiaddr: newRendezvous.value.multiaddr
    });
    console.log(info);
    const response = await proxy.$axios.post('/rendezvous', info, {
      headers: {
        'Content-Type': 'application/json'// 设置请求头，告诉服务器发送的是 JSON 数据
      }
    });
    console.log(response);
    rendezvousList.value.push(response.data);
    newRendezvous.value.multiaddr = '';
  } catch (error) {
    console.error('Error adding rendezvous:', error);
  }
}

async function deleteRendezvous(rendezvous) {
  try {
    const info = JSON.stringify({
      id: rendezvous.id,
      multiaddr: rendezvous.multiaddr
    });
    console.log(info);
    await proxy.$axios.delete('/rendezvous?id='+rendezvous.id);
    rendezvousList.value = rendezvousList.value.filter(rdz => rdz.id !== rendezvous.id);
  } catch (error) {
    console.error('Error deleting rendezvous:', error);
  }
}
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

form {
  display: flex;
  gap: 10px;
  justify-content: center;
  margin-top: 20px;
}

input[type="radio"] {
  margin-right: 5px;
}
</style>