# 🚀 Rust Blockchain Secure Wallet - 部署指南

**版本**: v1.0  
**更新日期**: 2025-11-03  
**适用环境**: 生产环境 + 测试环境

---

## 📋 目录

1. [环境要求](#环境要求)
2. [快速部署](#快速部署)
3. [生产环境部署](#生产环境部署)
4. [配置说明](#配置说明)
5. [安全检查清单](#安全检查清单)
6. [故障排查](#故障排查)

---

## 🔧 环境要求

### 系统要求

| 项目 | 最低要求 | 推荐配置 |
|------|---------|---------|
| OS | Ubuntu 20.04+ / CentOS 8+ | Ubuntu 22.04 LTS |
| CPU | 2核 | 4核+ |
| 内存 | 4GB | 8GB+ |
| 磁盘 | 20GB | 50GB+ SSD |
| 网络 | 10Mbps | 100Mbps+ |

### 软件依赖

```bash
# Rust工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install stable
rustc --version  # 需要 1.70+

# Node.js (前端)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
node --version  # 需要 16+

# Docker和Docker Compose
sudo apt-get update
sudo apt-get install docker.io docker-compose
docker --version
```

---

## 🚀 快速部署

### 方式1: Docker Compose（推荐）

```bash
# 1. 克隆项目
git clone https://github.com/your-org/rust-blockchain-wallet.git
cd rust-blockchain-wallet

# 2. 配置环境变量
cp Rust-Blockchain-Secure-Wallet/.env.example Rust-Blockchain-Secure-Wallet/.env
# 编辑.env文件，填写必要配置

# 3. 启动所有服务
docker-compose up -d

# 4. 检查状态
docker-compose ps
docker-compose logs -f defi-wallet

# 5. 访问服务
# API: http://localhost:8080
# 前端: http://localhost:3000
# Prometheus: http://localhost:9091
# Grafana: http://localhost:3000 (admin/admin)
```

### 方式2: 手动部署

```bash
# 1. 后端
cd Rust-Blockchain-Secure-Wallet

# 配置环境
cp .env.example .env
vim .env  # 填写配置

# 编译
cargo build --release

# 运行
./target/release/defi-hot-wallet

# 2. 前端
cd "../Wallet front-end/blockchain-wallet-ui"

# 安装依赖
npm install

# 构建
npm run build

# 部署（使用nginx或serve）
sudo npm install -g serve
serve -s build -p 3000
```

---

## 🏭 生产环境部署

### Step 1: 环境准备

```bash
# 创建部署目录
sudo mkdir -p /opt/blockchain-wallet
cd /opt/blockchain-wallet

# 创建数据目录
sudo mkdir -p data keys logs backups

# 设置权限
sudo chmod 700 keys
sudo chmod 755 data logs backups
```

### Step 2: 配置环境变量

```bash
# 创建.env文件
sudo vim /opt/blockchain-wallet/.env
```

**必需配置**:
```bash
# 安全配置（⚠️ 必须修改）
WALLET_ENC_KEY=$(openssl rand -base64 32)
API_KEY=$(openssl rand -hex 32)
BCRYPT_COST=12

# 服务器配置
WALLET_HOST=0.0.0.0
WALLET_PORT=8080
RUST_LOG=info

# 数据库
WALLET_DATABASE_URL=sqlite:/opt/blockchain-wallet/data/wallet.db

# RPC配置（使用付费API）
ETHEREUM_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
BITCOIN_RPC_URL=https://blockstream.info/api

# Etherscan API
ETHERSCAN_API_KEY=YOUR_ETHERSCAN_API_KEY

# CORS（生产环境域名）
CORS_ALLOW_ORIGIN=https://your-domain.com

# 生产模式（⚠️ 重要）
DEV_MODE=0
DEV_PRINT_SECRETS=0
```

### Step 3: 编译发布版本

```bash
cd /opt/blockchain-wallet/Rust-Blockchain-Secure-Wallet

# 编译优化版本
cargo build --release

# 检查二进制
ls -lh target/release/defi-hot-wallet
# 应该看到一个~50MB的二进制文件
```

### Step 4: 创建Systemd服务

```bash
sudo vim /etc/systemd/system/blockchain-wallet.service
```

**服务配置**:
```ini
[Unit]
Description=Rust Blockchain Secure Wallet API
After=network.target

[Service]
Type=simple
User=wallet
Group=wallet
WorkingDirectory=/opt/blockchain-wallet/Rust-Blockchain-Secure-Wallet
EnvironmentFile=/opt/blockchain-wallet/.env
ExecStart=/opt/blockchain-wallet/Rust-Blockchain-Secure-Wallet/target/release/defi-hot-wallet
Restart=always
RestartSec=10s

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/blockchain-wallet/data /opt/blockchain-wallet/logs

[Install]
WantedBy=multi-user.target
```

**启动服务**:
```bash
# 创建用户
sudo useradd -r -s /bin/false wallet
sudo chown -R wallet:wallet /opt/blockchain-wallet

# 启动服务
sudo systemctl daemon-reload
sudo systemctl enable blockchain-wallet
sudo systemctl start blockchain-wallet

# 检查状态
sudo systemctl status blockchain-wallet
sudo journalctl -u blockchain-wallet -f
```

### Step 5: 配置Nginx反向代理

```bash
sudo vim /etc/nginx/sites-available/blockchain-wallet
```

**Nginx配置**:
```nginx
upstream wallet_backend {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 80;
    server_name your-domain.com;
    
    # 重定向到HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;
    
    # SSL证书（Let's Encrypt）
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    
    # 前端静态文件
    root /opt/blockchain-wallet/frontend/build;
    index index.html;
    
    # API代理
    location /api/ {
        proxy_pass http://wallet_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        
        # 超时设置
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
    
    # 前端路由（SPA）
    location / {
        try_files $uri $uri/ /index.html;
    }
    
    # 安全头
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
}
```

**启用配置**:
```bash
sudo ln -s /etc/nginx/sites-available/blockchain-wallet /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### Step 6: SSL证书（Let's Encrypt）

```bash
# 安装certbot
sudo apt-get install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo certbot renew --dry-run
```

---

## 📊 监控配置

### Prometheus配置

已包含在`monitoring/prometheus.yml`中：

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

# 告警规则
rule_files:
  - 'prometheus-alerts.yml'

scrape_configs:
  - job_name: 'wallet-api'
    static_configs:
      - targets: ['localhost:9090']
```

### Grafana仪表板

访问: `http://localhost:3000` (admin/admin)

**导入仪表板**:
1. 点击 "+" → "Import"
2. 输入ID: 1860 (Node Exporter)
3. 选择Prometheus数据源
4. 保存

---

## ✅ 安全检查清单

### 部署前检查

- [ ] ✅ .env文件中的密钥已更换（不使用默认值）
- [ ] ✅ DEV_MODE=0（生产环境）
- [ ] ✅ HTTPS已配置（SSL证书）
- [ ] ✅ 防火墙已配置（只开放80/443）
- [ ] ✅ API认证已启用
- [ ] ✅ CORS限制为生产域名
- [ ] ✅ 数据库文件权限正确（600）
- [ ] ✅ 日志不包含敏感信息
- [ ] ✅ 定期备份已配置
- [ ] ✅ 监控告警已配置

### 运行时检查

```bash
# 检查服务状态
sudo systemctl status blockchain-wallet

# 检查日志
sudo journalctl -u blockchain-wallet --since "1 hour ago"

# 检查API健康
curl https://your-domain.com/api/health

# 检查Prometheus指标
curl http://localhost:9090/metrics

# 检查磁盘空间
df -h
```

---

## 🔍 故障排查

### 常见问题

**问题1: 服务启动失败**
```bash
# 查看详细日志
sudo journalctl -u blockchain-wallet -n 100 --no-pager

# 常见原因:
# - .env文件缺失或格式错误
# - 端口被占用
# - 数据库文件权限问题
# - WALLET_ENC_KEY格式错误
```

**问题2: RPC连接失败**
```bash
# 测试RPC连接
curl -X POST https://eth.llamarpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# 检查防火墙
sudo ufw status
```

**问题3: 交易签名失败**
```bash
# 检查错误日志
grep "sign.*failed" /opt/blockchain-wallet/logs/wallet.log

# 常见原因:
# - 密码错误
# - master_key损坏
# - RPC网络问题
```

---

## 📁 目录结构

```
/opt/blockchain-wallet/
├── Rust-Blockchain-Secure-Wallet/
│   ├── target/release/
│   │   └── defi-hot-wallet          # 主程序
│   ├── src/                          # 源代码
│   └── Cargo.toml
├── .env                              # ⚠️ 环境配置（不提交Git）
├── data/
│   └── wallet.db                     # SQLite数据库
├── keys/                             # 密钥存储（⚠️ 严格权限）
├── logs/                             # 日志文件
├── backups/                          # 数据库备份
└── monitoring/
    ├── prometheus.yml
    └── prometheus-alerts.yml
```

---

## 🔄 更新和回滚

### 更新流程

```bash
# 1. 备份数据
sudo cp /opt/blockchain-wallet/data/wallet.db \
       /opt/blockchain-wallet/backups/wallet.db.$(date +%Y%m%d_%H%M%S)

# 2. 拉取新代码
cd /opt/blockchain-wallet
git pull origin main

# 3. 编译新版本
cd Rust-Blockchain-Secure-Wallet
cargo build --release

# 4. 停止服务
sudo systemctl stop blockchain-wallet

# 5. 替换二进制
sudo cp target/release/defi-hot-wallet /opt/blockchain-wallet/

# 6. 启动服务
sudo systemctl start blockchain-wallet

# 7. 验证
curl https://your-domain.com/api/health
```

### 回滚流程

```bash
# 1. 停止服务
sudo systemctl stop blockchain-wallet

# 2. 恢复旧版本二进制
sudo cp /opt/blockchain-wallet/backups/defi-hot-wallet.backup \
       /opt/blockchain-wallet/defi-hot-wallet

# 3. 恢复数据库（如需要）
sudo cp /opt/blockchain-wallet/backups/wallet.db.20251103_120000 \
       /opt/blockchain-wallet/data/wallet.db

# 4. 启动服务
sudo systemctl start blockchain-wallet
```

---

## 📊 监控和告警

### Prometheus

**访问**: `http://localhost:9091`

**关键指标**:
- `api_requests_total` - API请求总数
- `api_errors_total` - API错误总数
- `transaction_count` - 交易数量
- `wallet_count` - 钱包数量

### Grafana

**访问**: `http://localhost:3000`

**默认密码**: admin/admin（首次登录后修改）

**仪表板**:
- API性能监控
- 系统资源监控
- 交易统计
- 错误率追踪

---

## 🔐 安全最佳实践

### 密钥管理

```bash
# 生成强密钥
openssl rand -base64 32  # WALLET_ENC_KEY
openssl rand -hex 32     # API_KEY

# 存储密钥（使用环境变量或密钥管理服务）
# ⚠️ 绝不要将.env文件提交到Git
echo ".env" >> .gitignore
```

### 防火墙配置

```bash
# 只开放必要端口
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 22/tcp    # SSH (限制IP)
sudo ufw enable
```

### 定期备份

```bash
# 创建备份脚本
sudo vim /usr/local/bin/backup-wallet.sh
```

**备份脚本**:
```bash
#!/bin/bash
BACKUP_DIR="/opt/blockchain-wallet/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# 备份数据库
cp /opt/blockchain-wallet/data/wallet.db \
   $BACKUP_DIR/wallet.db.$DATE

# 压缩
gzip $BACKUP_DIR/wallet.db.$DATE

# 删除30天前的备份
find $BACKUP_DIR -name "wallet.db.*.gz" -mtime +30 -delete

echo "Backup completed: wallet.db.$DATE.gz"
```

**设置定时任务**:
```bash
sudo chmod +x /usr/local/bin/backup-wallet.sh
sudo crontab -e

# 每天凌晨2点备份
0 2 * * * /usr/local/bin/backup-wallet.sh
```

---

## 🧪 部署验证

### 健康检查

```bash
# API健康检查
curl https://your-domain.com/api/health

# 预期响应:
{
  "status": "ok",
  "timestamp": "2025-11-03T..."
}
```

### 功能测试

```bash
# 1. 创建测试钱包
curl -X POST https://your-domain.com/api/wallets \
  -H "Content-Type: application/json" \
  -H "X-API-KEY: your-api-key" \
  -d '{"name":"test","password":"Test123!@#"}'

# 2. 查询地址
curl https://your-domain.com/api/wallets/test/address?network=ethereum \
  -H "X-API-KEY: your-api-key"

# 3. 查询余额
curl https://your-domain.com/api/wallets/test/balance?network=ethereum \
  -H "X-API-KEY: your-api-key"
```

### 性能测试

```bash
# 使用ab进行压力测试
ab -n 1000 -c 10 https://your-domain.com/api/health

# 预期:
# - 95%请求 <200ms
# - 无错误
# - QPS >50
```

---

## 📝 维护清单

### 每日任务

- [ ] 检查服务状态
- [ ] 查看错误日志
- [ ] 监控系统资源

### 每周任务

- [ ] 审查监控告警
- [ ] 检查磁盘空间
- [ ] 验证备份完整性

### 每月任务

- [ ] 更新依赖包
- [ ] 安全补丁
- [ ] 性能优化

---

## 📞 支持和帮助

**文档**:
- 环境配置: `环境配置指南.md`
- API文档: `README.md`
- 故障排查: 见下节

**日志位置**:
- 应用日志: `/opt/blockchain-wallet/logs/`
- 系统日志: `journalctl -u blockchain-wallet`
- Nginx日志: `/var/log/nginx/`

---

**部署指南版本**: v1.0  
**最后更新**: 2025-11-03  
**维护者**: DevOps Team


<!-- Updated: 2025-11-07 - Documentation enhancement -->
