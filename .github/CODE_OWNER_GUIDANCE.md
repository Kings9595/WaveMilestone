# WaveMilestone Code Owner Guidance for Soroban Contracts

This document provides comprehensive guidance for code owners responsible for maintaining and reviewing Soroban smart contracts in the WaveMilestone repository.

## Overview

Code owners are responsible for ensuring the quality, security, and maintainability of the Soroban contract codebase. This includes:

- Reviewing and approving contract changes
- Ensuring security best practices
- Maintaining test coverage
- Managing contract dependencies
- Coordinating with the broader team

## Code Owner Responsibilities

### 1. Contract Review Process

#### Initial Review
- **Scope**: Every change to `contracts/wave_milestone/src/`
- **Focus Areas**:
  - Logic correctness and edge cases
  - Security implications
  - Performance considerations
  - Code style and conventions

#### Security Review
- **Mandatory for**: All changes that modify:
  - Storage patterns
  - Authentication logic
  - Token transfer mechanisms
  - Cross-contract calls
- **Check Points**:
  - No unauthorized access vectors
  - Proper input validation
  - Safe arithmetic operations
  - Event emission for all state changes

### 2. Contract-Specific Rules

#### Authentication First
Every public method must:
1. Call `require_auth()` before any state mutation
2. Verify maintainer status via WaveGuard
3. Validate recipient address matches issue record

#### Check-Effects-Interaction Pattern
1. **Check**: Validate all inputs and preconditions
2. **Effects**: Update contract state
3. **Interaction**: Transfer tokens (last operation)

#### Balance Protection
- Always check `amount <= pool.remaining_balance()` before transfers
- Never attempt transfers without sufficient balance
- Handle `InsufficientPoolBalance` gracefully

#### Duplicate Protection
- Use composite key `(repo_hash, issue_id)` for uniqueness
- Set `completed: true` atomically with claim recording
- Revert with `BountyAlreadyClaimed` before any token transfer

### 3. Testing Requirements

#### Unit Tests (`contracts/wave_milestone/src/test.rs`)
- Test individual function behavior
- Cover error paths and edge cases
- Validate input validation logic
- Test authentication flows

#### Integration Tests (`contracts/wave_milestone/tests/*.rs`)
- Test cross-contract interactions
- Simulate full lifecycle scenarios
- Use `MockToken` and `MockWaveGuard`
- Test real-world usage patterns

#### Test Coverage Expectations
All new features must include:
1. **Happy path** test confirming the feature works
2. **Error path** test(s) confirming each error condition
3. **Edge case** tests for boundary conditions

### 4. Code Style and Conventions

#### Rust Formatting
- 4-space indentation
- 120-character line limit
- Unix line endings
- Run `cargo fmt` before committing

#### Linting Requirements
- `cargo clippy -- -D warnings` must pass
- Follow `clippy.toml` strict lint levels
- No panics in production code
- No unsafe code

#### Import Organization
- Group imports by standard library, external crates, and local modules
- Use `#[allow(...)]` sparingly
- Prefer `use` statements over fully qualified paths

### 5. Dependency Management

#### Contract Dependencies
- Update `contracts/wave_milestone/Cargo.toml` carefully
- Test with new versions before merging
- Document breaking changes

#### Version Pinning
- Use explicit version constraints where possible
- Review updates quarterly
- Test on testnet before mainnet

### 6. Documentation Requirements

#### Contract Documentation
- Document all public methods with:
  - Clear purpose description
  - Parameter documentation
  - Return value documentation
  - Auth requirements

#### Architecture Documentation
- Update `ARCHITECTURE.md` for significant changes
- Document storage patterns
- Explain security decisions

#### User Documentation
- Update README for new features
- Add examples to documentation
- Update deployment guides

## Code Owner Workflow

### Reviewing a Pull Request

1. **Initial Check**
   - All tests pass
   - Code follows style guidelines
   - No linting errors

2. **Security Review**
   - Check for unauthorized access
   - Validate authentication flows
   - Review storage patterns
   - Test edge cases

3. **Functional Review**
   - Verify logic correctness
   - Check error handling
   - Validate performance characteristics
   - Ensure documentation is updated

4. **Final Approval**
   - Request changes if needed
   - Approve when satisfied
   - Merge after CI passes

### Managing Contract Changes

#### Major Changes
- Create a branch with descriptive name
- Write comprehensive tests
- Update documentation
- Get approval from primary code owner

#### Bug Fixes
- Write a failing test first
- Fix the bug
- Ensure no regressions
- Update tests if needed

#### Refactoring
- Ensure no functional changes
- Update tests accordingly
- Verify performance characteristics
- Document changes

## Contract-Specific Guidance

### WaveMilestoneContract

#### Public Methods
- `create_milestone_pool`: Validate maintainer auth, check WaveGuard, transfer funds
- `release_issue_bounty`: Verify auth, check duplicate, validate balance, transfer tokens
- `clawback_expired_funds`: Verify maintainer, check expiry, transfer remaining funds

#### View Methods
- `milestone_balance`: Return remaining balance
- `is_claimed`: Check if issue already claimed
- `milestone_info`: Return full pool metadata

#### Error Handling
- Use specific error codes for each failure case
- Ensure all error paths are tested
- Document error conditions in documentation

### Storage Patterns

#### Instance Storage
- Use for persistent, contract-lifetime data
- Bump TTL on every write
- Keep storage keys minimal

#### Temporary Storage
- Use for single-use, short-lived data
- Leverage TTL for automatic cleanup
- Minimize gas costs

#### Key Design
- Use composite keys for uniqueness
- Keep key structure simple
- Document key usage

## Communication with Code Owners

### When to Contact

1. **Before making changes**:
   - Major contract modifications
   - Breaking changes
   - New authentication patterns

2. **During development**:
   - Stuck on security issues
   - Need for design review
   - Questions about best practices

3. **After changes**:
   - Request feedback
   - Discuss improvements
   - Coordinate with other owners

### Communication Channels

- **GitHub Discussions**: For design questions
- **Pull Request Reviews**: For code feedback
- **Direct Messages**: For urgent issues
- **Team Meetings**: For planning and coordination

## Code Owner Tools and Scripts

### Development Tools
- `cargo build --release`: Build optimized WASM
- `cargo test -- --nocapture`: Run integration tests
- `cargo fmt --check`: Check formatting
- `cargo clippy -- -D warnings`: Check linting

### Testing Scripts
- `./scripts/test.sh`: Run all tests
- `./scripts/test.sh <test_name>`: Run specific test
- `./scripts/build.sh release`: Build optimized WASM

### Contract Analysis
- Soroban CLI for contract interaction
- Custom test scripts for edge cases
- Security scanning tools

## Code Owner Training

### New Code Owners
1. **Review existing contracts**:
   - Understand current implementation
   - Learn security patterns
   - Study test coverage

2. **Participate in reviews**:
   - Review incoming PRs
   - Provide feedback
   - Learn from peers

3. **Take ownership**:
   - Lead reviews
   - Mentor new contributors
   - Maintain documentation

### Ongoing Learning
- Read security advisories
- Study new Soroban features
- Review similar projects
- Attend team meetings

## Code Owner Escalation

### Security Issues
- Report immediately to maintainers
- Follow responsible disclosure process
- Coordinate with other owners for fixes

### Blocked Reviews
- Communicate blockers promptly
- Coordinate with other owners
- Find alternative solutions

### Disputes
- Discuss with involved parties
- Seek consensus
- Escalate to maintainers if needed

## Code Owner Metrics

### Review Quality
- Number of PRs reviewed per week
- Security issues found
- Code quality improvements
- Documentation updates

### Contract Health
- Test coverage percentage
- Linting error rate
- Build success rate
- Security vulnerability count

### Team Growth
- Mentorship activities
- Knowledge sharing
- Training participation

## References

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [WaveGuard Documentation](https://github.com/anomalyco/waveguard)
- [Rust Best Practices](https://github.com/rust-lang/api-guidelines)
- [Security Guidelines](SECURITY.md)
- [Contributing Guidelines](CONTRIBUTING.md)

## Code Owner Contact

For questions or coordination:
- Primary contact: @anomalyco
- GitHub Discussions: https://github.com/anomalyco/wave-milestone/discussions
- Security Issues: Follow SECURITY.md process

## Code Owner Checklist

### Before Merging
- [ ] All tests pass
- [ ] Code follows project conventions
- [ ] Security review complete
- [ ] Documentation updated
- [ ] No linting or type-checking errors
- [ ] Commit messages follow Conventional Commits
- [ ] PR checklist items completed

### After Merging
- [ ] Update code owner metrics
- [ ] Share learnings with team
- [ ] Plan next review cycle
- [ ] Coordinate with other owners
