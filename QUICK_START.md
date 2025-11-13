# ⚡ 快速启动指南

## 🚀 一分钟启动后端

### 第一步：启动服务器

```bash
# Linux/macOS/Git Bash
./start-for-frontend.sh

# Windows PowerShell
.\start-for-frontend.ps1
```

### 第二步：验证运行

```bash
# Linux/macOS/Git Bash
./check-server.sh

# Windows PowerShell
.\check-server.ps1
```

看到 ✅ 就成功了！

---

## 📋 常用命令速查

### 启动服务器
```bash
./start-for-frontend.sh          # 脚本启动（推荐）
cargo run --bin hot_wallet       # 手动启动（开发模式）
cargo run --release --bin hot_wallet  # 手动启动（生产模式）
```

### 检查状态
```bash
./check-server.sh                # 完整健康检查
curl http://localhost:8888/api/health  # 简单检查
```

### 停止服务器
```bash
# 按 Ctrl+C
# 或
ps aux | grep hot_wallet && kill <PID>
```

---

## 🔐 必需环境变量

启动脚本已自动设置，手动启动时需要：

```bash
export SERVER_PORT=8888
export CORS_ALLOW_ORIGIN=http://localhost:3000
export DATABASE_URL=sqlite://./wallets.db
export JWT_SECRET=dev_secret_key_change_in_production
export WALLET_ENC_KEY=RDU3il552PpK2r0VsYY4UuLClwVDlL2XQHSPFRdKrjA=
export API_KEY=dev_api_key_change_in_production
export RUST_LOG=info
```

---

## 🌐 服务器信息

| 项目 | 值 |
|------|-----|
| 地址 | `http://localhost:8888` |
| 健康检查 | `http://localhost:8888/api/health` |
| WebSocket | `ws://localhost:8888/api/anomaly-detection/events` |
| CORS | `http://localhost:3000` |

---

## 🧪 快速测试

```bash
# 1. 健康检查（应返回 200）
curl http://localhost:8888/api/health

# 2. 认证测试（应返回 401）
curl http://localhost:8888/api/wallets

# 3. 桥接API（应返回 401）
curl http://localhost:8888/api/bridge/history
```

---

## ⚠️ 常见问题

### 问题：服务器无法启动
```bash
# 检查是否缺少环境变量
./start-for-frontend.sh  # 脚本会自动设置
```

### 问题：端口被占用
```bash
# 查找占用进程
netstat -ano | findstr 8888
# 或
lsof -i :8888

# 修改端口
export SERVER_PORT=8889
```

### 问题：代理错误 (502)
```bash
# 禁用代理
unset http_proxy
unset HTTP_PROXY
```

---

## 📚 详细文档

- **完整启动指南**: [BACKEND_STARTUP_GUIDE.md](BACKEND_STARTUP_GUIDE.md)
- **前端对接文档**: [BACKEND_READY_FOR_FRONTEND.md](BACKEND_READY_FOR_FRONTEND.md)
- **API 文档**: [API_DOCUMENTATION.md](API_DOCUMENTATION.md)
- **环境变量**: [ENV_VARIABLES.md](ENV_VARIABLES.md)

---

## ✅ 检查清单

启动前确认：
- [ ] Git Bash / PowerShell 已打开
- [ ] 在项目根目录下
- [ ] 网络连接正常

启动后验证：
- [ ] 健康检查返回 200 OK
- [ ] 认证端点返回 401
- [ ] CORS 头正确设置

前端对接：
- [ ] 前端端口为 3000
- [ ] 前端 API URL 为 http://localhost:8888
- [ ] 前端代理配置正确

---

**🎯 现在可以启动前端了！**

```bash
cd <前端目录>
npm start
```

---

**最后更新**: 2025-11-01  
**版本**: 0.1.0


<!-- Updated: 2025-11-07 - Documentation enhancement -->
