<template>
  <div class="table-container">
    <h2>Use Services</h2>
    <div class="form-container">
      <form @submit.prevent="addService">
        <!-- 第一行：ID 和 Host 输入框 -->
        <div class="form-row">
          <!-- <input v-model="newService.id" placeholder="ID" required /> -->
          <input v-model="newService.peer_id" placeholder="Peer Id" required />
          <input v-model="newService.host" placeholder="Host" required />
        </div>
        <!-- 第二行：Port, Forwarder Port 输入框和提交按钮 -->
        <div class="form-row">
          <input v-model="newService.port" type="number" placeholder="Port" required />
          <input v-model="newService.forwarder_port" type="number" placeholder="Forwarder Port" required />
          <button type="submit">Add</button>
        </div>
      </form>
    </div>
    <table>
      <thead>
        <tr>
          <th class="peerid-column">Peer Id</th>
          <th>Host</th>
          <th>Port</th>
          <th>Forwarder Port</th>
          <th>Action</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(service, index) in services" :key="index">
          <td class="peerid-cell">{{ service.peer_id }}</td>
          <td>{{ service.host }}</td>
          <td>{{ service.port }}</td>
          <td>{{ service.forwarder_port }}</td>
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
// const newService = ref({ id: '', host: '', port: null, forwarderPort: null });
const newService = ref({ peer_id: '', host: '', port: null, forwarder_port: null });
const {proxy} = getCurrentInstance();

const error = ref(null);

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
        peer_id: null,
        id: null, // 或者您可以设置一个默认的 id
        host: 'default-host', // 默认值
        port: 0, // 默认值
        forwarder_port: 0, // 默认值
      };
    }

    return {
      id: item.id || null, // 如果 id 缺失，则赋予 null 或者默认值
      peer_id: item.peer_id || null,
      host: item.host || 'default-host', // 如果 host 缺失，则赋予默认值
      port: typeof item.port === 'number' ? item.port : 0, // 如果 port 不是数字或缺失，则赋予默认值
      forwarder_port: typeof item.forwarder_port === 'number' ? item.forwarder_port : 0, // 如果 forwarder_port 不是数字或缺失，则赋予默认值
    };
  });
}

onMounted(async () => {
  try {
    console.log("restart useservices");
    const response = await proxy.$axios.get('/use_service');
    services.value = checkAndAssignDefaults(response.data);
  } catch (error) {
    console.error('Error fetching use services:', error);
    this.error = error; // 将错误信息保存到响应式引用中，可以在模板中显示
  }
});


async function addService() {
  try {
      const info = JSON.stringify({
      id : 0,
      peer_id: newService.value.peer_id,
      host: newService.value.host,
      port: newService.value.port,
      forwarder_port: newService.value.forwarder_port
    });
    // console.log(info);
    const response = await proxy.$axios.post('/use_service', info,{
      headers: {
        'Content-Type': 'application/json'// 设置请求头，告诉服务器发送的是 JSON 数据
      }
    });
    console.log(response);
    services.value.push(response.data);
    newService.value.host = '';
    newService.value.port = null;
    newService.value.forwarder_port = null;
  } catch (error) {
    console.error('Error adding service:', error);
  }
}


// 修改删除函数以发送被删除条目的详细信息
async function deleteService(service) {
  try {
    // const info = JSON.stringify({
    //     id: service.id,
    //     peer_id: service.peer_id,
    //     host: service.host,
    //     port: service.port,
    //     forwarder_port: service.forwarder_port,
    // });
    // await proxy.$axios.delete('/use_service', {
    //     data: info,
    //     headers: {
    //       'Content-Type': 'application/json'// 设置请求头，告诉服务器发送的是 JSON 数据
    //     }
    // });
    await proxy.$axios.delete('/use_service?id='+service.id);
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

.form-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px; /* 行间距 */
  width: 100%;
}

.form-row {
  display: flex;
  justify-content: center;
  gap: 10px; /* 输入框之间的间距 */
  flex-wrap: wrap; /* 允许内容换行 */
}

.form-row input[type="text"],
.form-row input[type="number"] {
  flex-grow: 1;
  padding: 8px;
  margin-right: 10px; /* 输入框右边距 */
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 16px;
}

.form-row button {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  background-color: #007bff;
  color: white;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.3s ease;
  white-space: nowrap;
}

.form-row button:hover {
  background-color: #0056b3;
}

@media (max-width: 600px) {
  .form-row input[type="text"],
  .form-row input[type="number"] {
    margin-right: 0; /* 在小屏幕上去除右边距 */
  }
}

.peerid-column {
  width: 200px; /* 或者您希望的固定宽度 */
}

.peerid-cell {
  max-width: 200px; /* 与 .peerid-column 宽度一致 */
  overflow-wrap: break-word; /* 允许在单词内换行 */
  overflow-x: auto; /* 水平方向上的滚动条 */
}
</style>