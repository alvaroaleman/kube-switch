# About

A tiny CLI to quickly switch contexts and namespaces in a kubeconfig, optimized for speed.

# Quickstart

## Installation
```
brew install cargo && export PATH="$HOME/.cargo/bin:$PATH" && cargo install --git https://github.com/alvaroaleman/kube-switch.git && source <(kube-switch completion)
```

Alternatively, fish shell completion  is also available:
```
kube-switch completion fish|source -
```

## Usage

### Switch context
```
sc $contextname
```

### Change namesapce

```
cn $namespace
```
