# 重要说明：本文档已废弃（Custodial）。

本项目采用“非托管（Non-Custodial）”标准，后端不保存助记词或私钥。此 API 文档仅作为历史记录保留，不适用于当前实现。

请参考：
- 非托管架构与安全：README.md、SECURITY.md
- 前端创建流程示例（非托管）：docs/FRONTEND_WALLET_CREATION_EXAMPLE.tsx

---

# 鎵樼寮忛挶鍖?API 鏂囨。

## 馃幆 鎵樼寮忛挶鍖呯壒鐐?

### 涓庨潪鎵樼寮忛挶鍖呯殑鍖哄埆

| 鐗规€?| 鎵樼寮忥紙褰撳墠瀹炵幇锛?| 闈炴墭绠″紡锛堝 MetaMask锛?|
|------|-------------------|------------------------|
| **瀵嗛挜瀛樺偍** | 鏈嶅姟绔鐞?| 鐢ㄦ埛鏈湴绠＄悊 |
| **鍔╄璇?* | 鏈嶅姟绔敓鎴愬拰淇濆瓨 | 鐢ㄦ埛鑷淇濆瓨 |
| **瀹夊叏璐ｄ换** | 骞冲彴鎵挎媴 | 鐢ㄦ埛鎵挎媴 |
| **鏄撶敤鎬?* | 楂橈紙绫讳技閾惰璐︽埛锛?| 浣庯紙闇€瑕佸浠藉姪璁拌瘝锛?|
| **閫傜敤鍦烘櫙** | 鏂版墜銆佸皬棰濅氦鏄?| 楂樼骇鐢ㄦ埛銆佸ぇ棰濊祫浜?|

### 瀹夊叏澧炲己鎺柦

1. **API Key 璁よ瘉** - 鎵€鏈夋晱鎰熸搷浣滈渶瑕侀獙璇?
2. **鎿嶄綔瀹¤** - 璁板綍鎵€鏈夊叧閿搷浣滄棩蹇?
3. **浜屾纭** - 鎻愮幇/杞处闇€瑕侀澶栫‘璁?
4. **2FA 鏀寔** - 鍙€夌殑鍙屽洜绱犺璇?
5. **椋庢帶绯荤粺** - AI 寮傚父妫€娴嬮泦鎴?

---

## 馃摗 API 绔偣

### 1. 鍒涘缓閽卞寘 (POST /api/wallets)

**璇锋眰浣?*:
```json
{
  "name": "MyWallet",              // 閽卞寘鍚嶇О锛堝敮涓€锛?
  "password": "StrongPass123!",    // 瀵嗙爜锛堟渶灏?浣嶏級
  "quantum_safe": false,           // 鏄惁鍚敤閲忓瓙瀹夊叏
  "two_factor_enabled": false      // 鏄惁鍚敤2FA锛堝彲閫夛級
}
```

**鎴愬姛鍝嶅簲** (201):
```json
{
  "wallet": {
    "id": "uuid-123",
    "name": "MyWallet",
    "address": "0x1234...5678",
    "quantum_safe": false,
    "created_at": "2025-10-29T12:00:00Z",
    "wallet_type": "Custodial-HD",
    "two_factor_enabled": false
  },
  "address": "0x1234...5678",
  "warning": "鈿狅笍 杩欐槸鎵樼寮忛挶鍖咃紝绉侀挜鐢卞钩鍙板畨鍏ㄤ繚绠°€傝濡ュ杽淇濈鎮ㄧ殑鐧诲綍瀵嗙爜銆?
}
```

**閿欒鍝嶅簲**:
- `400` - 閽卞寘鍚嶇О宸插瓨鍦?
- `400` - 瀵嗙爜寮哄害涓嶈冻
- `401` - API Key 楠岃瘉澶辫触

---

### 2. 鑾峰彇閽卞寘淇℃伅 (GET /api/wallets/{wallet_id})

**鍝嶅簲**:
```json
{
  "id": "uuid-123",
  "name": "MyWallet",
  "address": "0x1234...5678",
  "quantum_safe": false,
  "created_at": "2025-10-29T12:00:00Z",
  "last_activity": "2025-10-29T15:30:00Z",
  "security": {
    "two_factor_enabled": false,
    "last_password_change": "2025-10-29T12:00:00Z",
    "login_attempts": 0
  }
}
```

---

### 3. 瀵煎嚭閽卞寘澶囦唤 (POST /api/wallets/{wallet_id}/export)

**鈿狅笍 楂樺害鏁忔劅鎿嶄綔 - 闇€瑕佸閲嶉獙璇?*

**璇锋眰浣?*:
```json
{
  "password": "StrongPass123!",
  "two_factor_code": "123456",     // 濡傛灉鍚敤浜?FA
  "confirmation": "I understand"   // 鐢ㄦ埛纭
}
```

**鍝嶅簲**:
```json
{
  "success": true,
  "backup": {
    "wallet_id": "uuid-123",
    "address": "0x1234...5678",
    "encrypted_key": "...",        // 鍔犲瘑鍚庣殑绉侀挜
    "created_at": "2025-10-29T12:00:00Z",
  },
  "warning": "馃攼 璇峰皢姝ゅ浠戒俊鎭繚瀛樺湪瀹夊叏鐨勫湴鏂癸紒浠讳綍鎷ユ湁姝や俊鎭殑浜洪兘鍙互鎺у埗鎮ㄧ殑閽卞寘銆?,
  "audit_log_id": "audit-456"      // 瀹¤鏃ュ織ID
}
```

---

### 4. 楠岃瘉閽卞寘鍚嶇О鍞竴鎬?(GET /api/wallets/check-name?name=xxx)

**鐢ㄤ簬鍓嶇瀹炴椂楠岃瘉**

**鍝嶅簲**:
```json
{
  "name": "MyWallet",
  "available": false,
  "message": "閽卞寘鍚嶇О宸茶浣跨敤锛岃閫夋嫨鍏朵粬鍚嶇О"
}
```

---

### 5. 鑾峰彇瀹¤鏃ュ織 (GET /api/wallets/{wallet_id}/audit-logs)

**鍝嶅簲**:
```json
{
  "wallet_id": "uuid-123",
  "logs": [
    {
      "id": "audit-456",
      "action": "export_wallet",
      "timestamp": "2025-10-29T15:30:00Z",
      "ip_address": "192.168.1.1",
      "user_agent": "Mozilla/5.0...",
      "status": "success"
    }
  ],
  "total": 15
}
```

---

## 馃敀 瀹夊叏鏈哄埗

### API Key 璁よ瘉

鎵€鏈夋晱鎰熸搷浣滈渶瑕佸湪璇锋眰澶翠腑鎻愪緵 API Key锛?

```http
Authorization: Bearer YOUR_API_KEY
```

鎴栵細

```http
X-API-Key: YOUR_API_KEY
```

### 鎿嶄綔瀹¤

鎵€鏈夊叧閿搷浣滆嚜鍔ㄨ褰曪細
- 鍒涘缓閽卞寘
- 瀵煎嚭澶囦唤
- 淇敼瀵嗙爜
- 杞处鎿嶄綔
- 鐧诲綍灏濊瘯

### 浜屾纭鏈哄埗

楂橀闄╂搷浣滐紙濡傚鍑恒€佸ぇ棰濊浆璐︼級闇€瑕侊細
1. 瀵嗙爜楠岃瘉
2. 2FA 楠岃瘉锛堝鏋滃惎鐢級
3. 鐢ㄦ埛鏄庣‘纭

---

## 馃帹 鍓嶇闆嗘垚鎸囧崡

### 1. 閽卞寘鍒涘缓娴佺▼

```typescript
// 1. 妫€鏌ュ悕绉板敮涓€鎬э紙瀹炴椂锛?
const checkName = async (name: string) => {
  const res = await fetch(`/api/wallets/check-name?name=${name}`);
  const data = await res.json();
  return data.available;
};

// 2. 鏄剧ず鎵樼寮忛挶鍖呰鏄?
const showCustodialWarning = () => {
  // 鍦ㄥ垱寤哄脊绐椾腑鏄剧ず锛?
  // "鈿狅笍 杩欐槸鎵樼寮忛挶鍖?
  //  鈥?绉侀挜鐢卞钩鍙板畨鍏ㄤ繚绠?
  //  鈥?绫讳技閾惰璐︽埛锛屾棤闇€澶囦唤鍔╄璇?
  //  鈥?璇峰Ε鍠勪繚绠℃偍鐨勭櫥褰曞瘑鐮?
};

// 3. 鍒涘缓閽卞寘
const createWallet = async (name: string, password: string) => {
  const res = await fetch('/api/wallets', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, password, quantum_safe: false })
  });
  return await res.json();
};
```

### 2. 鍓嶇鏍￠獙瑙勫垯

```typescript
const validateWalletName = (name: string) => {
  // 1. 闀垮害: 3-32 瀛楃
  if (name.length < 3 || name.length > 32) {
    return '閽卞寘鍚嶇О闀垮害搴斿湪 3-32 涓瓧绗︿箣闂?;
  }
  
  // 2. 鍏佽鐨勫瓧绗? 瀛楁瘝銆佹暟瀛椼€佷笅鍒掔嚎銆佽繛瀛楃
  if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
    return '閽卞寘鍚嶇О鍙兘鍖呭惈瀛楁瘝銆佹暟瀛椼€佷笅鍒掔嚎鍜岃繛瀛楃';
  }
  
  // 3. 涓嶈兘浠ユ暟瀛楀紑澶?
  if (/^\d/.test(name)) {
    return '閽卞寘鍚嶇О涓嶈兘浠ユ暟瀛楀紑澶?;
  }
  
  return null; // 楠岃瘉閫氳繃
};

const validatePassword = (password: string) => {
  // 1. 鏈€灏忛暱搴?8 浣?
  if (password.length < 8) {
    return '瀵嗙爜闀垮害鑷冲皯 8 浣?;
  }
  
  // 2. 鑷冲皯鍖呭惈涓€涓暟瀛?
  if (!/\d/.test(password)) {
    return '瀵嗙爜蹇呴』鍖呭惈鑷冲皯涓€涓暟瀛?;
  }
  
  // 3. 鑷冲皯鍖呭惈涓€涓瓧姣?
  if (!/[a-zA-Z]/.test(password)) {
    return '瀵嗙爜蹇呴』鍖呭惈鑷冲皯涓€涓瓧姣?;
  }
  
  return null; // 楠岃瘉閫氳繃
};
```

### 3. 鍒涘缓寮圭獥UI寤鸿

```tsx
<Modal title="鍒涘缓鎵樼寮忛挶鍖?>
  {/* 鎵樼寮忚鏄?*/}
  <Alert type="info">
    <h4>鈿狅笍 鎵樼寮忛挶鍖呰鏄?/h4>
    <ul>
      <li>鉁?绉侀挜鐢卞钩鍙板畨鍏ㄤ繚绠★紝鏃犻渶澶囦唤鍔╄璇?/li>
      <li>鉁?绫讳技閾惰璐︽埛锛屼娇鐢ㄦ洿渚挎嵎</li>
      <li>鈿狅笍 璇峰Ε鍠勪繚绠℃偍鐨勭櫥褰曞瘑鐮?/li>
      <li>馃挕 閫傚悎鏂版墜鍜屽皬棰濅氦鏄?/li>
    </ul>
  </Alert>
  
  {/* 閽卞寘鍚嶇О杈撳叆 */}
  <Input
    label="閽卞寘鍚嶇О"
    value={name}
    onChange={handleNameChange}
    onBlur={checkNameAvailability}  // 澶辩劍鏃舵鏌ュ敮涓€鎬?
    error={nameError}
    hint="3-32涓瓧绗︼紝浠呮敮鎸佸瓧姣嶃€佹暟瀛椼€佷笅鍒掔嚎鍜岃繛瀛楃"
  />
  
  {/* 瀵嗙爜杈撳叆 */}
  <Input
    type="password"
    label="鐧诲綍瀵嗙爜"
    value={password}
    onChange={handlePasswordChange}
    error={passwordError}
    hint="鑷冲皯8浣嶏紝鍖呭惈瀛楁瘝鍜屾暟瀛?
  />
  
  {/* 瀵嗙爜寮哄害鎸囩ず鍣?*/}
  <PasswordStrength password={password} />
  
  {/* 閲忓瓙瀹夊叏閫夐」 */}
  <Checkbox
    label="鍚敤閲忓瓙瀹夊叏锛堝疄楠屾€э級"
    checked={quantumSafe}
    onChange={setQuantumSafe}
  />
  
  {/* 2FA 閫夐」 */}
  <Checkbox
    label="鍚敤鍙屽洜绱犺璇侊紙鎺ㄨ崘锛?
    checked={twoFactorEnabled}
    onChange={setTwoFactorEnabled}
  />
  
  <Button onClick={handleCreate}>鍒涘缓閽卞寘</Button>
</Modal>
```

---

## 馃攳 鍓嶇瀹炴椂楠岃瘉绀轰緥

```typescript
// 瀹炴椂妫€鏌ラ挶鍖呭悕绉?
const [nameStatus, setNameStatus] = useState<{
  checking: boolean;
  available: boolean | null;
  message: string;
}>({
  checking: false,
  available: null,
  message: ''
});

const checkWalletName = debounce(async (name: string) => {
  // 1. 鍏堝仛鍓嶇楠岃瘉
  const error = validateWalletName(name);
  if (error) {
    setNameStatus({
      checking: false,
      available: false,
      message: error
    });
    return;
  }
  
  // 2. 妫€鏌ュ悗绔敮涓€鎬?
  setNameStatus({ ...nameStatus, checking: true });
  try {
    const res = await fetch(`/api/wallets/check-name?name=${name}`);
    const data = await res.json();
    setNameStatus({
      checking: false,
      available: data.available,
      message: data.message
    });
  } catch (error) {
    setNameStatus({
      checking: false,
      available: null,
      message: '缃戠粶閿欒锛岃绋嶅悗閲嶈瘯'
    });
  }
}, 500); // 500ms 闃叉姈
```

---

## 馃洝锔?瀹夊叏鏈€浣冲疄璺?

### 1. 瀵嗙爜瀹夊叏
- 鉁?鏈€灏忛暱搴?8 浣?
- 鉁?蹇呴』鍖呭惈瀛楁瘝鍜屾暟瀛?
- 鉁?鎺ㄨ崘浣跨敤鐗规畩瀛楃
- 鉁?涓嶄娇鐢ㄥ父瑙佸瘑鐮?
- 鉁?瀹氭湡鎻愰啋鐢ㄦ埛鏇存敼瀵嗙爜

### 2. 鎿嶄綔瀹¤
- 鉁?璁板綍鎵€鏈夋晱鎰熸搷浣?
- 鉁?淇濆瓨 IP 鍦板潃鍜?User-Agent
- 鉁?寮傚父鐧诲綍鎻愰啋
- 鉁?瀵煎嚭鎿嶄綔闇€瑕侀偖浠堕€氱煡

### 3. 椋庨櫓鎺у埗
- 鉁?寮傚父鐧诲綍妫€娴?
- 鉁?澶ч杞处闄愬埗
- 鉁?棰戠箒鎿嶄綔闄愭祦
- 鉁?AI 寮傚父妫€娴嬮泦鎴?

### 4. 2FA锛堝弻鍥犵礌璁よ瘉锛?
- 鉁?鏀寔 TOTP锛圙oogle Authenticator锛?
- 鉁?鏀寔 SMS 楠岃瘉鐮?
- 鉁?鏀寔閭楠岃瘉鐮?
- 鉁?鎭㈠鐮佹満鍒?

---

## 馃搳 涓?AI 寮傚父妫€娴嬮泦鎴?

鎵樼寮忛挶鍖呯殑鎵€鏈変氦鏄撹嚜鍔ㄩ€氳繃 AI 寮傚父妫€娴嬬郴缁燂細

```json
// 鍒涘缓閽卞寘鏃惰嚜鍔ㄥ惎鐢ㄥ紓甯告娴?
{
  "wallet_id": "uuid-123",
  "anomaly_detection": {
    "enabled": true,
    "risk_level": "medium",
    "auto_block_threshold": 0.9
  }
}
```

杞处鏃剁殑寮傚父妫€娴嬫祦绋嬶細
1. 鐢ㄦ埛鍙戣捣杞处
2. AI 绯荤粺璇勪及椋庨櫓
3. 楂橀闄╀氦鏄撹嚜鍔ㄦ嫤鎴垨闇€瑕佷簩娆＄‘璁?
4. 璁板綍瀹¤鏃ュ織
5. 寮傚父閫氱煡鐢ㄦ埛

---

## 馃幆 鎬荤粨

鎵樼寮忛挶鍖呯殑鏍稿績浼樺娍锛?
- 鉁?**鏄撶敤鎬ч珮** - 鏃犻渶澶囦唤鍔╄璇?
- 鉁?**骞冲彴淇濋殰** - 涓撲笟鐨勫畨鍏ㄥ洟闃熺鐞?
- 鉁?**闆嗘垚 AI** - 鑷姩寮傚父妫€娴嬪拰椋庢帶
- 鉁?**閫傚悎鏂版墜** - 闄嶄綆浣跨敤闂ㄦ

鐢ㄦ埛闇€瑕佺悊瑙ｇ殑椋庨櫓锛?
- 鈿狅笍 渚濊禆骞冲彴瀹夊叏鎬?
- 鈿狅笍 骞冲彴鏈夎闂潈闄?
- 鈿狅笍 闇€瑕佷俊浠绘湇鍔℃彁渚涙柟

鎺ㄨ崘浣跨敤鍦烘櫙锛?
- 鉁?鏂版墜鐢ㄦ埛
- 鉁?灏忛浜ゆ槗
- 鉁?棰戠箒浜ゆ槗
- 鉁?DeFi 浣撻獙

涓嶆帹鑽愪娇鐢ㄥ満鏅細
- 鉂?澶ч璧勪骇瀛樺偍
- 鉂?闀挎湡鎸佹湁
- 鉂?鏋佸害娉ㄩ噸闅愮

---

## 馃摓 鏀寔

濡傛湁闂锛岃鏌ョ湅锛?
- API 鏂囨。锛歚/docs/API_ENDPOINTS_COMPLETE.md`
- 瀹夊叏鏈€浣冲疄璺碉細`/docs/API_BEST_PRACTICES.md`
- 鍓嶇楠岃瘉鎸囧崡锛歚/docs/FRONTEND_VERIFICATION.md`

