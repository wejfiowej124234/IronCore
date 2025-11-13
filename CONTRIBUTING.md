# 🤝 Contributing to Blockchain Wallet

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

---

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)

---

## 📜 Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all.

### Expected Behavior

- ✅ Be respectful and inclusive
- ✅ Welcome newcomers
- ✅ Focus on constructive criticism
- ✅ Accept responsibility for mistakes
- ✅ Show empathy towards others

### Unacceptable Behavior

- ❌ Harassment or discrimination
- ❌ Trolling or insulting comments
- ❌ Publishing private information
- ❌ Other unprofessional conduct

---

## 🎯 How to Contribute

### Types of Contributions

We welcome many types of contributions:

1. **🐛 Bug Reports**
   - Found a bug? Report it!
   - Include steps to reproduce
   - Provide error messages/logs

2. **💡 Feature Requests**
   - Have an idea? Share it!
   - Explain the use case
   - Consider implementation approach

3. **📝 Documentation**
   - Improve existing docs
   - Add missing documentation
   - Fix typos or clarifications

4. **🔧 Code Contributions**
   - Bug fixes
   - New features
   - Performance improvements
   - Refactoring

5. **🧪 Testing**
   - Add test cases
   - Improve test coverage
   - Fix failing tests

---

## 🛠️ Development Setup

### Prerequisites

```bash
# Required
- Rust 1.70+ (rustup recommended)
- Node.js 16+ (for frontend)
- Git

# Optional but recommended
- VS Code or RustRover
- Rust Analyzer extension
```

---

### Initial Setup

#### 1. Fork the Repository

```bash
# Click "Fork" on GitHub, then:
git clone https://github.com/YOUR_USERNAME/Rust-Secure-Wallet-AI
cd Rust-Blockchain-Secure-Wallet
```

#### 2. Add Upstream Remote

```bash
git remote add upstream https://github.com/DarkCrab-Rust/Rust-Secure-Wallet-AI
```

#### 3. Install Dependencies

```bash
# Backend
cargo build

# Frontend
cd ../Wallet\ front-end/blockchain-wallet-ui
npm install
```

#### 4. Set Up Environment

```bash
# Copy environment template
cp .env.example .env

# Edit with your values
export WALLET_ENC_KEY="your_key_here"
export API_KEY="your_api_key"
```

#### 5. Run Tests

```bash
# Backend tests
cargo test --all-features

# Frontend tests
npm test
```

---

## 📐 Coding Standards

### Rust Code Style

#### 1. Use `cargo fmt`

```bash
# Format all code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check
```

**Required**: All code must pass `cargo fmt`

---

#### 2. Use `cargo clippy`

```bash
# Run clippy
cargo clippy --all-features

# Deny warnings (CI standard)
cargo clippy -- -D warnings
```

**Required**: Zero clippy warnings

---

#### 3. Naming Conventions

```rust
// ✅ Good
struct UserAccount { }
fn create_wallet() { }
const MAX_RETRIES: u32 = 3;

// ❌ Bad
struct user_account { }  // snake_case for types
fn CreateWallet() { }    // PascalCase for functions
const maxRetries: u32 = 3;  // camelCase for constants
```

---

#### 4. Error Handling

```rust
// ✅ Good - Return Result
fn process_transaction() -> Result<TxHash, WalletError> {
    let tx = build_transaction()?;
    Ok(broadcast(tx)?)
}

// ❌ Bad - unwrap() or panic!()
fn process_transaction() -> TxHash {
    let tx = build_transaction().unwrap();  // ❌
    broadcast(tx).expect("failed")  // ❌
}

// Exception: Tests can use unwrap()
#[test]
fn test_something() {
    let result = do_something().unwrap();  // ✅ OK in tests
}
```

---

#### 5. Documentation

```rust
/// Creates a new HD wallet with the given parameters.
///
/// # Arguments
///
/// * `name` - Unique wallet identifier
/// * `password` - User password for encryption
/// * `quantum_safe` - Enable experimental quantum resistance
///
/// # Returns
///
/// Returns `Ok(Wallet)` with the created wallet and mnemonic phrase.
///
/// # Errors
///
/// Returns `WalletError` if:
/// - Wallet name already exists
/// - Password is too weak
/// - Mnemonic generation fails
///
/// # Example
///
/// ```
/// let wallet = create_wallet("my_wallet", "strong_password", false)?;
/// println!("Mnemonic: {}", wallet.mnemonic);
/// ```
pub fn create_wallet(
    name: &str,
    password: &str,
    quantum_safe: bool
) -> Result<Wallet, WalletError> {
    // Implementation
}
```

---

#### 6. Module Organization

```rust
// Good module structure
pub mod wallet_manager {
    pub mod lifecycle;  // Create/delete
    pub mod transactions;  // Send/receive
    pub mod balance;  // Balance queries
    
    pub use lifecycle::create_wallet;
    pub use transactions::send_transaction;
}
```

---

### TypeScript Code Style

#### 1. Use Prettier

```bash
# Format code (if configured)
npm run format

# Check formatting
npm run format:check
```

---

#### 2. Type Safety

```typescript
// ✅ Good - Explicit types
interface Wallet {
    name: string;
    address: string;
    balance: string;
}

function createWallet(name: string): Promise<Wallet> {
    // Implementation
}

// ❌ Bad - Using 'any'
function createWallet(name: any): any {  // ❌
    // Implementation
}
```

---

#### 3. React Best Practices

```typescript
// ✅ Good - Functional components with hooks
const WalletPage: React.FC = () => {
    const [wallets, setWallets] = useState<Wallet[]>([]);
    
    useEffect(() => {
        fetchWallets();
    }, []);
    
    return <div>...</div>;
};

// ❌ Bad - Class components (avoid)
class WalletPage extends React.Component {  // ❌
    // Old style
}
```

---

## 🧪 Testing Guidelines

### Backend Testing

#### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wallet_name_validation() {
        assert!(validate_wallet_name("valid_name").is_ok());
        assert!(validate_wallet_name("").is_err());
        assert!(validate_wallet_name("a".repeat(100)).is_err());
    }
    
    #[test]
    fn test_amount_validation() {
        assert!(validate_amount("1.5").is_ok());
        assert!(validate_amount("-1").is_err());
        assert!(validate_amount("not_a_number").is_err());
    }
}
```

---

#### 2. Integration Tests

```rust
// tests/integration_test.rs
use defi_hot_wallet::*;

#[tokio::test]
async fn test_wallet_creation_flow() {
    let config = test_config();
    let manager = WalletManager::new(config).await;
    
    // Create wallet
    let wallet = manager.create_wallet("test_wallet", "password").await?;
    assert!(!wallet.mnemonic.is_empty());
    
    // Verify it exists
    let wallets = manager.list_wallets().await?;
    assert!(wallets.contains(&"test_wallet".to_string()));
}
```

---

#### 3. Test Coverage

```bash
# Generate coverage report
cargo tarpaulin --out Html

# View report
open tarpaulin-report.html
```

**Goal**: Maintain 80%+ coverage

---

### Frontend Testing

```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { WalletPage } from './WalletPage';

describe('WalletPage', () => {
    it('should render wallet list', () => {
        render(<WalletPage />);
        expect(screen.getByText('My Wallets')).toBeInTheDocument();
    });
    
    it('should create new wallet', async () => {
        render(<WalletPage />);
        
        const createButton = screen.getByText('Create Wallet');
        fireEvent.click(createButton);
        
        // More assertions...
    });
});
```

---

## 🔄 Pull Request Process

### 1. Create a Branch

```bash
# Update main
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name

# Or for bugfix
git checkout -b fix/bug-description
```

---

### 2. Make Changes

```bash
# Make your changes
# Add tests
# Update documentation

# Stage changes
git add .

# Commit with meaningful message
git commit -m "feat: add wallet export feature"
```

---

### 3. Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <description>

[optional body]

[optional footer]
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance

**Examples**:
```bash
feat: add Bitcoin Taproot support
fix: resolve memory leak in wallet manager
docs: update API documentation for bridge endpoints
test: add unit tests for encryption module
```

---

### 4. Push and Create PR

```bash
# Push to your fork
git push origin feature/your-feature-name

# Go to GitHub and create Pull Request
```

---

### 5. PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-reviewed the code
- [ ] Commented complex code
- [ ] Updated documentation
- [ ] Added tests
- [ ] All tests pass
- [ ] No new warnings

## Testing
How to test the changes

## Screenshots (if applicable)
```

---

### 6. Code Review Process

1. **Automated Checks**
   - ✅ CI tests must pass
   - ✅ Code must be formatted (`cargo fmt`)
   - ✅ No clippy warnings
   - ✅ Test coverage maintained

2. **Manual Review**
   - At least 1 reviewer approval
   - Address all comments
   - Update if requested

3. **Merge**
   - Squash and merge (preferred)
   - Delete branch after merge

---

## 🐛 Issue Reporting

### Bug Report Template

```markdown
**Describe the bug**
A clear description of the bug

**To Reproduce**
Steps to reproduce:
1. Go to '...'
2. Click on '...'
3. See error

**Expected behavior**
What should happen

**Actual behavior**
What actually happens

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.70]
- Browser: [e.g., Chrome 100]

**Logs/Screenshots**
If applicable

**Additional context**
Any other relevant information
```

---

### Feature Request Template

```markdown
**Is your feature request related to a problem?**
A clear description of the problem

**Describe the solution you'd like**
What you want to happen

**Describe alternatives**
Other solutions you've considered

**Additional context**
Mockups, examples, etc.
```

---

## 🏅 Recognition

Contributors will be:
- ✅ Listed in CONTRIBUTORS.md
- ✅ Mentioned in release notes
- ✅ Credited in documentation

---

## 📞 Getting Help

**Questions?**
- 💬 GitHub Discussions
- 📧 Create an issue with "question" label
- 📖 Check documentation first

**Need guidance?**
- Look at existing PRs
- Ask in discussions
- Reach out to maintainers

---

## 🎯 Good First Issues

Looking to start contributing? Check issues labeled:
- `good first issue`
- `help wanted`
- `documentation`

---

## 📚 Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Axum Documentation](https://docs.rs/axum/)
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)

---

## 🙏 Thank You!

Your contributions make this project better for everyone. We appreciate your time and effort!

---

**Contributing Guidelines Version**: 1.0  
**Last Updated**: November 6, 2025  
**Maintainer**: @DarkCrab-Rust

<!-- Updated: 2025-11-07 - Documentation enhancement -->
