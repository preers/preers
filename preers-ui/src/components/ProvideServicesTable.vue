<template>
  <div class="table-container">
    <h2>Provide Services</h2>
    <div class="form-container">
      <form @submit.prevent="addService">
        <div class="form-row">
          <input v-model="newService.host" placeholder="Host" required />
        </div>
        <div class="form-row">
          <input v-model="newService.port" type="number" placeholder="Port" required />
        </div>
        <button type="submit">Add</button>
      </form>
    </div>
    <table>
      <thead>
        <tr>
          <th>Host</th>
          <th>Port</th>
          <th>Action</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(service, index) in services" :key="index">
          <td>{{ service.host }}</td>
          <td>{{ service.port }}</td>
          <td>
            <button @click="deleteService(service)">Delete</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted, getCurrentInstance } from 'vue';
import axios from 'axios';

const services = ref([]);
const newService = ref({ host: '', port: null });
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
        host: 'default-host',
        port: 0,
      };
    }

    return {
      id: typeof item.id === 'number' ? item.id : null,
      host: typeof item.host === 'string' ? item.host : 'default-host',
      port: typeof item.port === 'number' ? item.port : 0,
    };
  });
}


onMounted(async () => {
  try {
    // console.log("restart provide services");
    const response = await proxy.$axios.get('/provide_service');
    services.value = checkAndAssignDefaults(response.data);
  } catch (error) {
    console.error('Error fetching provide services:', error);
  }
});

// // 添加服务
async function addService() {
  try {
      const info = JSON.stringify({
      id : 0,
      host: newService.value.host,
      port: newService.value.port
    });
      const response = await proxy.$axios.post('/provide_service', info, {
        headers: {
        'Content-Type': 'application/json'// 设置请求头，告诉服务器发送的是 JSON 数据
      }
    });
    //console.log(response);
    // 更新服务列表
    services.value.push(response.data);
    newService.value.host = '';
    newService.value.port = null;
  } catch (error) {
    console.error('Error adding service:', error);
  }
}


//删除服务
async function deleteService(service) {
  try {
    // const info = JSON.stringify({
    //     id: service.id,
    //     host: service.host,
    //     port: service.port
    // });
    await proxy.$axios.delete('/provide_service?id='+service.id);
    // 从服务列表中移除
    services.value = services.value.filter(s => s.id !== service.id);
  } catch (error) {
    console.error('Error deleting service:', error);
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