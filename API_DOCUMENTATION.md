# 🔐 DeFi Hot Wallet - 后端 API 接口文档

## 📋 目录

- [基本信息](#基本信息)
- [认证方式](#认证方式)
- [API 端点](#api-端点)
  - [健康检查](#健康检查)
  - [钱包管理](#钱包管理)
  - [交易操作](#交易操作)
  - [跨链桥接](#跨链桥接)
  - [用户认证](#用户认证)
  - [监控指标](#监控指标)

---

## 基本信息

- **Base URL (开发)**: `http://localhost:8080`
- **Base URL (生产)**: `https://api.example.com`
- **API 版本**: `v0.1.0`
- **协议**: HTTP/HTTPS
- **数据格式**: JSON
- **字符编码**: UTF-8

### CORS 配置

- **允许的源**: `http://localhost:3000` (可通过 `CORS_ALLOW_ORIGIN` 环境变量配置)
- **允许的方法**: `GET`, `POST`, `DELETE`, `PUT`, `PATCH`, `OPTIONS`
- **允许的头**: `Authorization`, `Content-Type`, `Accept`, `Origin`
- **支持凭证**: ✅ 是 (`credentials: 'include'`)

---

## 认证方式

### API Key 认证

大多数敏感端点需要在请求头中包含 API Key：

```http
Authorization: Bearer <your_api_key>
```

**示例**:
```javascript
fetch('http://localhost:8080/api/wallets', {
  headers: {
    'Authorization': 'Bearer your_api_key_here',
    'Content-Type': 'application/json'
  }
})
```

---

## API 端点

### 健康检查

#### `GET /api/health`

检查服务器健康状态

**请求**:
```http
GET /api/health HTTP/1.1
Host: localhost:8080
```

**响应** `200 OK`:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2025-10-31T14:00:00Z"
}
```

---

### 钱包管理

#### `POST /api/wallets`

创建新钱包

**请求**:
```http
POST /api/wallets HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer <api_key>

{
  "name": "my_wallet",
  "quantum_safe": false
}
```

**响应** `201 Created`:
```json
{
  "name": "my_wallet",
  "addresses": {
    "ethereum": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
    "bitcoin": "bc1q..."
  },
  "created_at": "2025-10-31T14:00:00Z"
}
```

**错误响应** `400 Bad Request`:
```json
{
  "error": "Invalid wallet name",
  "code": "INVALID_INPUT"
}
```

---

#### `GET /api/wallets`

获取所有钱包列表

**请求**:
```http
GET /api/wallets HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**响应** `200 OK`:
```json
{
  "wallets": [
    {
      "name": "wallet1",
      "addresses": {
        "ethereum": "0x...",
        "bitcoin": "bc1q..."
      },
      "created_at": "2025-10-31T14:00:00Z"
    },
    {
      "name": "wallet2",
      "addresses": {
        "ethereum": "0x...",
        "bitcoin": "bc1q..."
      },
      "created_at": "2025-10-31T14:01:00Z"
    }
  ]
}
```

---

#### `DELETE /api/wallets/:name`

删除指定钱包

**请求**:
```http
DELETE /api/wallets/my_wallet HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**响应** `200 OK`:
```json
{
  "message": "Wallet 'my_wallet' deleted successfully"
}
```

**错误响应** `404 Not Found`:
```json
{
  "error": "Wallet not found",
  "code": "NOT_FOUND"
}
```

---

#### `GET /api/wallets/:name/balance`

查询钱包余额

**请求**:
```http
GET /api/wallets/my_wallet/balance?network=ethereum HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**查询参数**:
- `network` (可选): 网络名称 (`ethereum`, `bitcoin`, `polygon`, `bsc`)

**响应** `200 OK`:
```json
{
  "wallet": "my_wallet",
  "network": "ethereum",
  "balance": "1.5",
  "currency": "ETH",
  "usd_value": "4500.00"
}
```

---

#### `GET /api/wallets/:name/history`

获取交易历史

**请求**:
```http
GET /api/wallets/my_wallet/history?limit=10&offset=0 HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**查询参数**:
- `limit` (可选): 返回数量，默认 10
- `offset` (可选): 偏移量，默认 0
- `network` (可选): 过滤网络

**响应** `200 OK`:
```json
{
  "wallet": "my_wallet",
  "transactions": [
    {
      "tx_hash": "0xabc123...",
      "from": "0x...",
      "to": "0x...",
      "amount": "0.5",
      "currency": "ETH",
      "status": "confirmed",
      "timestamp": "2025-10-31T14:00:00Z",
      "network": "ethereum"
    }
  ],
  "total": 25,
  "limit": 10,
  "offset": 0
}
```

---

#### `GET /api/wallets/:name/backup`

备份钱包（非托管策略）

**说明**:
- 非托管设计中，助记词仅在“创建钱包”时显示一次，不会存储在服务器。
- 生产环境默认不支持导出助记词；测试环境会返回明文（用于自动化测试）。

**请求**:
```http
GET /api/wallets/my_wallet/backup HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**生产环境响应** `400 Bad Request`:
```json
{
  "error": "Backup not supported",
  "code": "BACKUP_NOT_SUPPORTED"
}
```

**测试环境响应** `200 OK`:
```json
{
  "version": "v1-test",
  "alg": "PLAINTEXT",
  "kek_id": null,
  "nonce": "",
  "ciphertext": "YmFzZTY0LWVuY29kZWQtbW5lbW9uaWMvc2VlZC1waHJhc2U=",
  "wallet": "my_wallet"
}
```

字段说明:
- `version`: 备份对象版本（测试环境为 `v1-test`）
- `alg`: 算法，测试环境为 `PLAINTEXT`（明文以 base64 返回）
- `ciphertext`: base64 编码的助记词字节
- `nonce`: 测试环境为空字符串
- `wallet`: 钱包名称

**⚠️ 安全警告**:
- 生产环境不导出助记词是行业最佳实践（参考 MetaMask/Trust Wallet）。
- 测试环境返回明文仅用于自动化测试，请勿用于真实生产数据。

---

#### `POST /api/wallets/restore`

从助记词恢复钱包

**请求**:
```http
POST /api/wallets/restore HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer <api_key>

{
  "name": "restored_wallet",
  "mnemonic": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
}
```

**响应** `201 Created`:
```json
{
  "name": "restored_wallet",
  "addresses": {
    "ethereum": "0x...",
    "bitcoin": "bc1q..."
  },
  "message": "Wallet restored successfully"
}
```

---

### 交易操作

#### `POST /api/wallets/:name/send`

发送交易

**请求**:
```http
POST /api/wallets/my_wallet/send HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer <api_key>

{
  "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
  "amount": "0.1",
  "network": "ethereum",
  "password": "user_password"
}
```

**请求参数**:
- `to` (必需): 接收地址
- `amount` (必需): 金额
- `network` (必需): 网络 (`ethereum`, `bitcoin`, `polygon`, `bsc`)
- `password` (必需): 钱包密码（用于解密私钥）
- `gas_price` (可选): Gas 价格（以太坊）
- `fee_rate` (可选): 费率（比特币）

**响应** `200 OK`:
```json
{
  "tx_hash": "0xabc123...",
  "from": "0x...",
  "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
  "amount": "0.1",
  "currency": "ETH",
  "status": "pending",
  "network": "ethereum",
  "timestamp": "2025-10-31T14:00:00Z"
}
```

**错误响应** `400 Bad Request`:
```json
{
  "error": "Insufficient funds",
  "code": "INSUFFICIENT_FUNDS"
}
```

**错误响应** `401 Unauthorized`:
```json
{
  "error": "Invalid password",
  "code": "INVALID_PASSWORD"
}
```

---

#### `POST /api/wallets/:name/send_multi_sig`

多签名交易

**请求**:
```http
POST /api/wallets/my_wallet/send_multi_sig HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer <api_key>

{
  "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
  "amount": "1.0",
  "signatures": ["sig1", "sig2", "sig3"],
  "threshold": 2
}
```

**响应** `200 OK`:
```json
{
  "tx_hash": "0xdef456...",
  "status": "pending",
  "message": "Multi-sig transaction submitted"
}
```

---

### 跨链桥接

#### `POST /api/bridge`

发起跨链桥接

**请求**:
```http
POST /api/bridge HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Authorization: Bearer <api_key>

{
  "from_wallet": "my_wallet",
  "from_chain": "ethereum",
  "to_chain": "polygon",
  "token": "USDC",
  "amount": "100.0"
}
```

**请求参数**:
- `from_wallet` (必需): 源钱包名称
- `from_chain` (必需): 源链 (`ethereum`, `polygon`, `bsc`)
- `to_chain` (必需): 目标链 (`ethereum`, `polygon`, `bsc`)
- `token` (必需): 代币符号 (`USDC`, `USDT`, `ETH`, 等)
- `amount` (必需): 金额

**响应** `200 OK`:
```json
{
  "bridge_id": "bridge_123456",
  "from_chain": "ethereum",
  "to_chain": "polygon",
  "token": "USDC",
  "amount": "100.0",
  "status": "pending",
  "estimated_time": "5-10 minutes",
  "from_tx_hash": "0xabc...",
  "timestamp": "2025-10-31T14:00:00Z"
}
```

**错误响应** `400 Bad Request`:
```json
{
  "error": "Unsupported chain",
  "code": "UNSUPPORTED_CHAIN"
}
```

---

#### `GET /api/bridge/:id`

查询桥接状态

**请求**:
```http
GET /api/bridge/bridge_123456 HTTP/1.1
Host: localhost:8080
Authorization: Bearer <api_key>
```

**响应** `200 OK`:
```json
{
  "bridge_id": "bridge_123456",
  "status": "completed",
  "from_chain": "ethereum",
  "to_chain": "polygon",
  "from_tx_hash": "0xabc...",
  "to_tx_hash": "0xdef...",
  "amount": "100.0",
  "token": "USDC",
  "created_at": "2025-10-31T14:00:00Z",
  "completed_at": "2025-10-31T14:05:00Z"
}
```

**状态值**:
- `pending`: 等待处理
- `processing`: 处理中
- `completed`: 已完成
- `failed`: 失败

---

### 用户认证

#### `POST /api/auth/register`

用户注册

**请求**:
```http
POST /api/auth/register HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "name": "John Doe"
}
```

**响应** `201 Created`:
```json
{
  "user_id": "user_123",
  "email": "user@example.com",
  "name": "John Doe",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "refresh_token_here"
}
```

---

#### `POST /api/auth/login`

用户登录

**请求**:
```http
POST /api/auth/login HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```

**响应** `200 OK`:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "refresh_token_here",
  "user": {
    "user_id": "user_123",
    "email": "user@example.com",
    "name": "John Doe"
  }
}
```

**错误响应** `401 Unauthorized`:
```json
{
  "error": "Invalid email or password",
  "code": "INVALID_CREDENTIALS"
}
```

---

#### `GET /api/auth/me`

获取当前用户信息

**请求**:
```http
GET /api/auth/me HTTP/1.1
Host: localhost:8080
Authorization: Bearer <jwt_token>
```

**响应** `200 OK`:
```json
{
  "user_id": "user_123",
  "email": "user@example.com",
  "name": "John Doe",
  "created_at": "2025-10-01T00:00:00Z"
}
```

---

### 监控指标

#### `GET /api/metrics`

获取 Prometheus 格式的监控指标

**请求**:
```http
GET /api/metrics HTTP/1.1
Host: localhost:8080
```

**响应** `200 OK`:
```text
# HELP wallet_transactions_total Total number of transactions
# TYPE wallet_transactions_total counter
wallet_transactions_total{network="ethereum"} 1234

# HELP wallet_balance_eth Current ETH balance
# TYPE wallet_balance_eth gauge
wallet_balance_eth{wallet="wallet1"} 1.5
```

---

## 错误代码

| 代码 | HTTP 状态 | 说明 |
|------|-----------|------|
| `INVALID_INPUT` | 400 | 无效的输入参数 |
| `INSUFFICIENT_FUNDS` | 400 | 余额不足 |
| `INVALID_PASSWORD` | 401 | 密码错误 |
| `UNAUTHORIZED` | 401 | 未授权 |
| `NOT_FOUND` | 404 | 资源不存在 |
| `RATE_LIMIT_EXCEEDED` | 429 | 超出速率限制 |
| `INTERNAL_ERROR` | 500 | 服务器内部错误 |
| `NETWORK_ERROR` | 503 | 网络错误 |

---

## 速率限制

### IP 级别限制

- **每秒请求数**: 10 req/s
- **突发请求数**: 20 req
- **限制策略**: Token Bucket

**超出限制响应** `429 Too Many Requests`:
```json
{
  "error": "Rate limit exceeded",
  "code": "RATE_LIMIT_EXCEEDED",
  "retry_after": 1
}
```

### 账户级别限制

- **敏感操作（交易/桥接）**: 5 req/min
- **查询操作**: 60 req/min

---

## 安全最佳实践

### 🔒 前端建议

1. **使用 HTTPS**: 生产环境必须使用 HTTPS
2. **不要在前端存储私钥**: 所有私钥操作在后端完成
3. **Token 管理**: 
   - JWT token 存储在 `localStorage` 或 `sessionStorage`
   - 设置合理的过期时间（建议 1 小时）
   - 使用 refresh token 自动刷新
4. **敏感数据**: 
   - 助记词、私钥不要存储在浏览器
   - 密码不要明文传输
5. **CORS**: 确保后端 CORS 配置正确

### 🛡️ 请求示例（前端）

```javascript
// 创建钱包
async function createWallet(name) {
  try {
    const response = await fetch('http://localhost:8080/api/wallets', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${getApiKey()}`
      },
      body: JSON.stringify({
        name: name,
        quantum_safe: false
      })
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error);
    }
    
    const data = await response.json();
    console.log('钱包创建成功:', data);
    return data;
  } catch (error) {
    console.error('创建钱包失败:', error);
    throw error;
  }
}

// 发送交易
async function sendTransaction(walletName, to, amount, network, password) {
  try {
    const response = await fetch(`http://localhost:8080/api/wallets/${walletName}/send`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${getApiKey()}`
      },
      body: JSON.stringify({
        to,
        amount,
        network,
        password
      })
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error);
    }
    
    const data = await response.json();
    console.log('交易发送成功:', data);
    return data;
  } catch (error) {
    console.error('发送交易失败:', error);
    throw error;
  }
}

// 获取 API Key（示例）
function getApiKey() {
  // 从环境变量或配置中获取
  return process.env.REACT_APP_API_KEY || 'your_api_key_here';
}
```

---

## WebSocket 支持（计划中）

### 连接

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/transactions');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('新交易:', data);
};
```

---

## 环境变量配置

### 后端必需环境变量

```bash
# 数据库
DATABASE_URL=sqlite://./wallets.db

# API 密钥（用于认证）
WALLET_API_KEY=your_secret_api_key_here

# JWT 密钥
JWT_SECRET=your_jwt_secret_here

# CORS 配置
CORS_ALLOW_ORIGIN=http://localhost:3000

# 服务器配置
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# 区块链 RPC（可选）
ETH_RPC_URL=https://eth.llamarpc.com
POLYGON_RPC_URL=https://polygon-rpc.com
BSC_RPC_URL=https://bsc-dataseed.binance.org

# 加密密钥（用于加密存储）
WALLET_ENC_KEY=your_32_byte_encryption_key_base64

# 安全配置
PBKDF2_ITERATIONS=600000
BCRYPT_COST=12
```

---

## 📞 联系支持

- **技术支持**: support@example.com
- **文档**: https://docs.example.com
- **GitHub**: https://github.com/your-org/defi-hot-wallet

---

## 📜 更新日志

### v0.1.0 (2025-10-31)
- ✅ 初始版本
- ✅ 钱包管理 API
- ✅ 交易操作 API
- ✅ 跨链桥接 API
- ✅ 用户认证 API
- ✅ 安全加固（110+ 安全问题修复）

---

**最后更新**: 2025-10-31  
**API 版本**: v0.1.0  
**文档版本**: 1.0.0


<!-- Updated: 2025-11-07 - Documentation enhancement -->
