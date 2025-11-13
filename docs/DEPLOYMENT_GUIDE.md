# 🚀 部署指南

## 环境要求

### 后端
- Rust 1.70+
- 操作系统：Linux/macOS/Windows
- 内存：2GB+
- 存储：1GB+

### 前端
- Node.js 16+
- npm 8+

---

## 快速部署

### 1. 后端部署

```bash
# 进入后端目录
cd Rust-Blockchain-Secure-Wallet

# 编译发布版本
cargo build --release

# 运行
./target/release/blockchain-wallet-server
```

**环境变量**:
```bash
export API_KEY="your_secure_api_key"
export DEV_MODE=0  # 生产环境设为0
export CORS_ORIGIN="https://your-frontend-domain.com"
export TRUST_PROXY_HEADERS=1  # 如果在反向代理后
```

---

### 2. 前端部署

```bash
# 进入前端目录
cd "Wallet front-end/blockchain-wallet-ui"

# 安装依赖
npm install

# 构建生产版本
npm run build

# 部署build目录到Web服务器
```

**环境变量**:
```bash
REACT_APP_API_URL=https://api.your-domain.com
REACT_APP_API_KEY=your_api_key
```

---

## Nginx配置示例

```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    # 前端
    location / {
        root /var/www/blockchain-wallet-ui/build;
        try_files $uri /index.html;
    }
    
    # API代理
    location /api {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
    
    # WebSocket
    location /ws {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

---

## Docker部署

### Dockerfile（后端）
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/blockchain-wallet-server /usr/local/bin/
CMD ["blockchain-wallet-server"]
```

### Dockerfile（前端）
```dockerfile
FROM node:16 as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/build /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
```

---

## 监控配置

### 1. 日志收集
```bash
# 后端日志
export RUST_LOG=info

# 输出到文件
./blockchain-wallet-server 2>&1 | tee /var/log/wallet-backend.log
```

### 2. 性能监控
- 建议使用Prometheus + Grafana
- 监控指标：请求数、响应时间、错误率

### 3. 错误追踪
- 前端：集成Sentry
- 后端：配置错误日志告警

---

## 安全配置

### 必须配置
1. **HTTPS**: 强制使用HTTPS
2. **防火墙**: 只开放必要端口
3. **API Key**: 使用强随机密钥
4. **CORS**: 严格限制来源
5. **速率限制**: 启用IP限速

### 推荐配置
1. **备份**: 定期备份数据
2. **监控**: 配置告警规则
3. **日志**: 保留30天日志
4. **更新**: 定期更新依赖

---

## 健康检查

### 端点
```
GET /api/health
```

### 监控脚本
```bash
#!/bin/bash
while true; do
    curl -f http://127.0.0.1:8080/api/health || \
    echo "Backend down!" | mail -s "Alert" admin@example.com
    sleep 60
done
```

---

## 故障排查

### 后端无法启动
1. 检查端口8080是否被占用
2. 检查日志文件
3. 验证环境变量

### 前端无法连接后端
1. 检查CORS配置
2. 验证API_URL配置
3. 检查网络连接

### 性能问题
1. 检查数据库索引
2. 启用缓存
3. 增加并发限制

---

**版本**: v1.0  
**更新**: 2025-11-03

